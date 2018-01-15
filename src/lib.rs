extern crate libc;

use std;
use std::ffi::CStr;

pub type RawDartHandle = *mut libc::c_void;
pub type RawDartNativeArguments = *mut libc::c_void;
pub type RawDartWeakPersistentHandle = *mut libc::c_void;

#[repr(C)]
pub struct Any(RawDartHandle);

#[repr(C)]
pub struct Empty(RawDartHandle);

#[repr(C)]
pub struct Error(RawDartHandle);

#[repr(C)]
pub struct Library(RawDartHandle);

#[repr(C)]
pub struct String(RawDartHandle);

#[repr(C)]
pub struct SendPort(RawDartHandle);

#[repr(C)]
pub struct NativeArguments(RawDartNativeArguments);

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Port(i64);

pub type NativeFunction = extern "C" fn(args: NativeArguments);
pub type NativeEntryResolver = extern "C" fn(name: String, argc: i32, auto_setup_scope: *mut bool) -> Option<NativeFunction>;
pub type WeakPersistentHandleFinalizer = extern "C" fn(isolate_callback_data: *const libc::c_void, handle: RawDartWeakPersistentHandle, peer: *mut libc::c_void);
pub type NativeMessageHandler = extern "C" fn(dest_port: Port, message: *const RawCObject);

////////////////////////////////////////////////////////////////////////////////////////////////////

#[cfg_attr(target_os = "windows", link(name = "dart", kind = "static"))]
extern {
    fn Dart_IsError(handle: RawDartHandle) -> bool;
    fn Dart_IsString(object: RawDartHandle) -> bool;
    fn Dart_IsLibrary(object: RawDartHandle) -> bool;

    fn Dart_Null() -> RawDartHandle;
    fn Dart_NewApiError(message: *const u8) -> RawDartHandle;
    fn Dart_ToString(object: RawDartHandle) -> RawDartHandle;

    fn Dart_StringToCString(object: RawDartHandle, chars: *mut *const u8) -> RawDartHandle;
    fn Dart_NewStringFromUTF8(chars: *const u8, length: usize) -> Any;

    fn Dart_SetNativeResolver(library: RawDartHandle, resolver: NativeEntryResolver, symbol_lookup: *const libc::c_void) -> RawDartHandle;

    fn Dart_SetReturnValue(args: RawDartNativeArguments, handle: RawDartHandle);

    fn Dart_PropagateError(handle: RawDartHandle) -> !;

    fn Dart_NewSendPort(port: Port) -> Any;
    fn Dart_NewNativePort(name: *const u8, handler: NativeMessageHandler, handle_concurrently: bool) -> Port;
    fn Dart_CloseNativePort(port: Port) -> bool;
    fn Dart_PostCObject(port: Port, message: *mut RawCObject) -> bool;
    fn Dart_PostInteger(port: Port, message: i64) -> bool;
    fn Dart_Post(port: Port, object: RawDartHandle) -> bool;
}

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait DartHandle {
    fn as_raw(&self) -> RawDartHandle;
    fn as_any(&self) -> Any;
}

pub trait DartType {
    fn is_type(handle: &Any) -> bool;
    fn from(handle: &Any) -> Self;
    fn typename() -> &'static str;
}

////////////////////////////////////////////////////////////////////////////////////////////////////

impl DartHandle for Any {
    fn as_raw(&self) -> RawDartHandle { self.0 }
    fn as_any(&self) -> Any { Any(self.as_raw()) }
}

impl DartHandle for Error {
    fn as_raw(&self) -> RawDartHandle { self.0 }
    fn as_any(&self) -> Any { Any(self.as_raw()) }
}

impl Any {
    pub fn is_error(&self) -> bool { unsafe { Dart_IsError(self.0) } }

    fn to_string_raw(&self) -> Any { Any( unsafe { Dart_ToString(self.0) } ) }

    pub fn as_object<T : DartType>(&self) -> Result<T, Error> {
        if T::is_type(self) {
            return Ok(T::from(self));
        } else if self.is_error() {
            return Err(Error(self.0));
        } else {
            // Type cast error
            // We shouldn't get into a cast loop with this
            let to_string: Result<String, Error> = self.to_string_raw().as_object();
            return Err(match to_string {
                Ok(dart_string) => {
                    let formatted_error = format!(
                        "Cast failure, wanted {} got {}",
                        T::typename(),
                        dart_string.as_str().unwrap_or(
                            std::string::String::from("Failed to convert to string")
                        )
                    );
                    Error( unsafe { Dart_NewApiError(formatted_error.as_bytes().as_ptr()) } )
                }
                Err(dart_error) => {
                    dart_error
                }
            });
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

impl DartHandle for Empty {
    fn as_raw(&self) -> RawDartHandle { self.0 }
    fn as_any(&self) -> Any { Any(self.as_raw()) }
}

impl DartType for Empty {
    fn is_type(handle: &Any) -> bool { !handle.is_error() }
    fn from(handle: &Any) -> Empty { Empty(handle.0) }
    fn typename() -> &'static str { return "None" }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

impl DartHandle for String {
    fn as_raw(&self) -> RawDartHandle { self.0 }
    fn as_any(&self) -> Any { Any(self.as_raw()) }
}

impl DartType for String {
    fn is_type(handle: &Any) -> bool { unsafe { Dart_IsString(handle.0) } }
    fn from(handle: &Any) -> String { String(handle.0) }
    fn typename() -> &'static str { return "String" }
}

impl String {
    pub fn as_str(&self) -> Result<std::string::String, Error> {
        let mut chars: *const u8 = 0 as *const u8;
        let result: Result<Empty, Error> = Any(unsafe {
            Dart_StringToCString(self.0, &mut chars)
        }).as_object();
        return match result {
            Ok(_) => Ok(unsafe {
                CStr::from_ptr(chars as *const i8).to_str().unwrap().to_string()
            }),
            Err(e) => Err(e)
        }
    }

    pub fn from_utf8(chars: &[u8]) -> Result<String, Error> {
        unsafe {
            Dart_NewStringFromUTF8(chars.as_ptr(), chars.len()).as_object()
        }
    }

    pub fn from_string(string: &std::string::String) -> Result<String, Error> { String::from_utf8(string.as_bytes()) }

    pub fn from_str(string: &str) -> Result<String, Error> { String::from_utf8(string.as_bytes()) }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

impl DartHandle for Library {
    fn as_raw(&self) -> RawDartHandle { self.0 }
    fn as_any(&self) -> Any { Any(self.as_raw()) }
}

impl DartType for Library {
    fn is_type(handle: &Any) -> bool { unsafe { Dart_IsLibrary(handle.0) } }
    fn from(handle: &Any) -> Library { Library(handle.0) }
    fn typename() -> &'static str { return "Library" }
}

impl Library {
    pub fn set_native_resolver(&self, resolver: NativeEntryResolver) -> Result<Empty, Error> {
        let result = unsafe { Dart_SetNativeResolver(self.0, resolver, 0 as *const libc::c_void) };
        return Any(result).as_object()
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

impl NativeArguments {
    pub fn set_return<A : DartHandle>(&self, value: A) {
        unsafe {
            Dart_SetReturnValue(self.0, value.as_raw())
        }
    }
}

impl DartHandle for SendPort {
    fn as_raw(&self) -> RawDartHandle { self.0 }
    fn as_any(&self) -> Any { Any(self.as_raw()) }
}

impl DartType for SendPort {
    // TODO: Better SendPort checking, maybe Dart_SendPortGetId?
    fn is_type(handle: &Any) -> bool { !handle.is_error() }
    fn from(handle: &Any) -> SendPort { SendPort(handle.0) }
    fn typename() -> &'static str { return "SendPort" }
}

impl Port {
    pub fn is_invalid(&self) -> bool { self.0 == 0 }

    pub fn close(&self) -> bool { unsafe { Dart_CloseNativePort(Port(self.0)) } }
    pub fn post_raw_cobject(&self, message: *mut RawCObject) -> bool { unsafe {
        Dart_PostCObject(Port(self.0), message)
    } }
    pub fn post_integer(&self, message: i64) -> bool { unsafe {
        Dart_PostInteger(Port(self.0), message)
    } }
    pub fn post_object<T : DartHandle>(&self, object: T) -> bool { unsafe {
        Dart_Post(Port(self.0), object.as_raw())
    } }

    pub fn as_send_port(&self) -> Result<SendPort, Error> {
        unsafe { Dart_NewSendPort(*self) }.as_object()
    }

    pub fn invalid() -> Port { Port(0) }
    pub fn make_native_port(name: &str, handler: NativeMessageHandler, handle_concurrently: bool) -> Option<Port> {
        let port = unsafe { Dart_NewNativePort(name.as_ptr(), handler, handle_concurrently) };
        if port.is_invalid() {
            None
        } else {
            Some(port)
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

#[repr(C)]
#[derive(Copy, Clone)]
pub enum DartTypedDataType {
    ByteData,
    Int8,
    Uint8,
    Uint8Clamped,
    Int16,
    Uint16,
    Int32,
    Uint32,
    Int64,
    Uint64,
    Float32,
    Float64,
    Float32x4,
    Invalid
}

#[repr(C)]
#[derive(Copy, Clone)]
pub enum DartCObjectType {
    Null,
    Bool,
    Int32,
    Int64,
    BigInt,
    Double,
    String,
    Array,
    TypedData,
    ExternalTypedData,
    SendPort,
    Capability,
    Unsupported,
}

#[repr(C)]
pub struct RawCObject {
    typ: DartCObjectType,
    value: DartCObjectValue
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union DartCObjectValue {
    null: (),
    bool: bool,
    int32: i32,
    int64: i64,
    double: f64,
    string: *const u8,
    big_int: CObjectValueBigInt,
    send_port: CObjectValueSendPort,
    capability: i64,
    array: CObjectValueArray,
    typed_data: CObjectValueTypedData,
    external_typed_data: CObjectValueExternalTypedData
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct CObjectValueBigInt {
    pub neg: bool,
    pub used: usize,
    pub digits: *mut RawCObject
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct CObjectValueSendPort {
    pub id: Port,
    pub origin_id: Port
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct CObjectValueArray {
    pub length: usize,
    pub values: *mut *mut RawCObject
}

impl CObjectValueArray {
    pub fn at(&self, index: usize) -> CObject {
        CObject::from(unsafe { *self.values.offset(index as isize) } as *const RawCObject)
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct CObjectValueTypedData {
    pub typ: DartTypedDataType,
    pub length: usize,
    pub values: *const u8
}

impl CObjectValueTypedData {
    pub fn as_slice<'a>(&self) -> &'a [u8] {
        unsafe {
            std::slice::from_raw_parts(self.values, self.length)
        }
    }
}

impl<'a> From<&'a [u8]> for CObjectValueTypedData {
    fn from(slice: &'a [u8]) -> CObjectValueTypedData {
        CObjectValueTypedData {
            values: slice.as_ptr(),
            length: slice.len(),
            typ: DartTypedDataType::Uint8
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct CObjectValueExternalTypedData {
    pub typ: DartTypedDataType,
    pub length: usize,
    pub values: *mut u8,
    pub peer: *const libc::c_void,
    pub callback: WeakPersistentHandleFinalizer
}

pub enum CObject {
    Null,
    Bool(bool),
    Int32(i32),
    Int64(i64),
    Double(f64),
    String(*const u8),
    BigInt(CObjectValueBigInt),
    SendPort(CObjectValueSendPort),
    Capability(i64),
    Array(CObjectValueArray),
    TypedData(CObjectValueTypedData),
    ExternalTypedData(CObjectValueExternalTypedData),
}

impl<'a> From<&'a RawCObject> for CObject {
    fn from(object: &RawCObject) -> Self {
        use dart::CObject::*;
        use dart::DartCObjectType as Type;

        unsafe {
            match object.typ {
                Type::Null => Null,
                Type::Bool => Bool(object.value.bool),
                Type::Int32 => Int32(object.value.int32),
                Type::Int64 => Int64(object.value.int64),
                Type::Double => Double(object.value.double),
                Type::String => String(object.value.string),
                Type::BigInt => BigInt(object.value.big_int),
                Type::SendPort => SendPort(object.value.send_port),
                Type::Capability => Capability(object.value.capability),
                Type::Array => Array(object.value.array),
                Type::TypedData => TypedData(object.value.typed_data),
                Type::ExternalTypedData => ExternalTypedData(object.value.external_typed_data),
                _ => panic!("Unsupported type hit!")
            }
        }
    }
}

impl<'a> From<&'a CObject> for RawCObject {
    fn from(object: &CObject) -> RawCObject {
        use dart::CObject::*;
        use dart::DartCObjectType as Type;

        match *object {
            Null => RawCObject { typ: Type::Null, value: DartCObjectValue { null: () } },
            Bool(v) => RawCObject { typ: Type::Bool, value: DartCObjectValue { bool: v } },
            Int32(v) => RawCObject { typ: Type::Int32, value: DartCObjectValue { int32: v } },
            Int64(v) => RawCObject { typ: Type::Int64, value: DartCObjectValue { int64: v } },
            Double(v) => RawCObject { typ: Type::Double, value: DartCObjectValue { double: v } },
            String(v) => RawCObject { typ: Type::String, value: DartCObjectValue { string: v } },
            BigInt(ref v) => RawCObject { typ: Type::BigInt, value: DartCObjectValue { big_int: *v } },
            SendPort(ref v) => RawCObject { typ: Type::SendPort, value: DartCObjectValue { send_port: *v } },
            Capability(v) => RawCObject { typ: Type::Capability, value: DartCObjectValue { capability: v } },
            Array(ref v) => RawCObject { typ: Type::Array, value: DartCObjectValue { array: *v } },
            TypedData(ref v) => RawCObject { typ: Type::TypedData, value: DartCObjectValue { typed_data: *v } },
            ExternalTypedData(ref v) => RawCObject { typ: Type::ExternalTypedData, value: DartCObjectValue { external_typed_data: *v } },
        }
    }
}

impl From<*const RawCObject> for CObject {
    fn from(object: *const RawCObject) -> CObject {
        CObject::from(unsafe { &*object })
    }
}

impl CObject {
    pub fn as_str(&self) -> Option<&str> {
        match *self {
            CObject::String(data) => Some(
                unsafe {
                    CStr::from_ptr(data as *const i8).to_str().unwrap()
                }
            ),
            _ => None
        }
    }

    pub fn as_raw(&self) -> RawCObject { RawCObject::from(self) }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

pub fn null() -> Any { Any( unsafe { Dart_Null() } ) }

pub fn unwrap<A : DartHandle, B : DartHandle>(result: Result<A, B>) -> Any {
    return match result {
        Ok(a) => a.as_any(),
        Err(b) => b.as_any()
    }
}

pub fn simplify_result<A : DartHandle>(result: Result<A, Error>) -> Result<Any, Error> {
    match result {
        Ok(a) => Ok(a.as_any()),
        Err(b) => Err(b)
    }
}

pub fn wrap_native_fn<A : DartHandle, T : FnOnce(&NativeArguments) -> Result<A, Error>>(args: &NativeArguments, func: T) {
    match func(args) {
        Ok(handle) => args.set_return(handle),
        Err(err) => unsafe { propagate_error(&err) }
    }
}

pub unsafe fn propagate_error(error: &Error) -> ! { Dart_PropagateError(error.0) }

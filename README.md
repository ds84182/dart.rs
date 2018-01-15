# dart.rs - Dart native extension library for Rust

NOTE: Be aware that this is my first Rust library. Constructive feedback would be appreciated.

## Features:

* Synchronous and Asynchronous native extension support

## Usage:

In Rust:

```rust
// Must be in the format <library name>_Init, where <library name> is the same as the dll/so (<library name>.dll, lib<library name>.so)
#[no_mangle]
#[allow(non_snake_case)]
pub extern "C" fn example_Init(library: dart::Any) -> dart::Any {
    return dart::unwrap(
        library
            .as_object()
            .and_then(|lib: dart::Library| lib.set_native_resolver(func_resolver)));
}

extern "C" fn func_resolver(name: dart::String, argc: i32, auto_setup_scope: *mut bool) -> Option<dart::NativeFunction> {
    use std::ops::Deref;

    match name.as_str() {
        Ok(str) => match str.deref() {
            "test_native" => Some(test_native_fn),
            "port_test" => Some(port_test_fn),
            _ => None
        },
        Err(e) => unsafe { dart::propagate_error(&e) }
    }
}

fn test_native_fn_inner(args: &dart::NativeArguments) -> Result<dart::String, dart::Error> {
    dart::String::from_str("Hello from Rust!")
}

extern "C" fn test_native_fn(args: dart::NativeArguments) { dart::wrap_native_fn(&args, test_native_fn_inner) }

fn port_test(args: &dart::NativeArguments) -> Result<dart::SendPort, dart::Error> {
    dart::Port::make_native_port("port_test", port_test_handler, true)
        .map(|port| port.as_send_port())
        .unwrap()
}

extern "C" fn port_test_handler(dest_port: dart::Port, raw_message: *const dart::RawCObject) {
    let message_root = dart::CObject::from(raw_message);

    use dart::CObject::*;

    if let Array(ref array) = message_root {
        println!("Got array {}", array.length);
        if let SendPort(ref reply_port) = array.at(0) {
            println!("Got send port");
            let payload = array.at(1);

            let response = match payload {
                Int32(v) => Int32(v * 2),
                Int64(v) => Int64(v * 2),
                Double(v) => Double(v * 2.0),
                _ => String("Cannot double given object".as_ptr())
            };

            reply_port.id.post_raw_cobject(&mut response.as_raw());
        } else {
            println!("Received improperly formatted message")
        }
    } else {
        println!("Received improperly formatted message")
    }
}

extern "C" fn port_test_fn(args: dart::NativeArguments) { dart::wrap_native_fn(&args, port_test) }
```

In Cargo.toml:

```toml
[lib]
# ...
crate-type = ["dylib"]
```

In Dart:

```dart
import 'dart-ext:example';
import 'dart:isolate';

String testNative() native 'test_native';
SendPort portTest() native 'port_test';

void main() {
    print(testNative());
    final recvPort = new ReceivePort();
    portTest().send([
        recvPort,
        3.2,
    ]);
    recvPort.first.then(print);
}
```

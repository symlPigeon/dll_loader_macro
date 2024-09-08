# DLL_LOADER_MACRO

从bindgen生成的FFI接口自动生成加载动态链接库中函数的代码。

```rust
// In `my_test.h`
MY_LIB_API int add(int a, int b);

//-------//

// In your Rust code:
generate_dll_loader!("my_test.h");

// And you will get:
pub struct DllLoader {
    /* PRIVATE FIELDS */
}
impl DllLoader {
    pub unsafe fn new(path: &str) -> Self;
    pub unsafe fn add(a: i32, b: i32) -> i32;
}
```

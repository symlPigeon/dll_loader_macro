use dll_loader_macro::generate_dll_loader;

generate_dll_loader!("tests/my_test.h");

#[test]
fn test_generate_dll_loader1() {
    let loader = unsafe { DllLoader::new("tests/my_c_dll.dll") };
    let value = unsafe {
        loader.add(40, 2)
    };
    assert_eq!(value as u32, ANSWER);
}

#[test]
#[should_panic]
fn test_generate_dll_loader2() {
    unsafe {
        let loader = DllLoader::new("tests/my_c_dll.dll");
        loader.this_will_crash();
    }
}

#[test]
fn test_generate_dll_loader3() {
    unsafe {
        let loader = DllLoader::new("tests/my_c_dll.dll");
        let mut my_struct = MyStruct {
            a: 20,
            b: 100
        };
        loader.change_struct(10, 101, &mut my_struct);
        assert_eq!(my_struct.a, 10);
        assert_eq!(my_struct.b, 101);
    }
}
extern crate proc_macro;
use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;

#[proc_macro]
pub fn generate_dll_loader(input: TokenStream) -> TokenStream {

    let input = parse_macro_input!(input as syn::LitStr);
    let input = input.value();

    // bindgen
    let bindings = bindgen::Builder::default()
        .header(input)
        .generate()
        .expect("Failed to generate bindings.");

    let tokens = syn::parse_str::<syn::File>(&bindings.to_string()).unwrap();

    let mut func_struct_def = Vec::new();
    let mut func_struct_init = Vec::new();
    let mut func_struct_impl = Vec::new();
    let mut func_struct_new = Vec::new();

    for item in tokens.items.iter() {
        if let syn::Item::ForeignMod(foreign_mod) = item {
            let abi = &foreign_mod.abi;
            let abi_name = abi.name.clone().unwrap();
            for item in foreign_mod.items.iter() {
                if let syn::ForeignItem::Fn(foreign_fn) = item {
                    let ident = &foreign_fn.sig.ident;
                    let inputs = &foreign_fn.sig.inputs;
                    let output = &foreign_fn.sig.output;

                    let def_ident = syn::Ident::new(&format!("addr_of_{}", ident), ident.span());
                    let u8str_ident =
                        syn::LitByteStr::new(ident.to_string().as_bytes(), ident.span());

                    let fn_call_args: Vec<_> = inputs
                        .iter()
                        .filter_map(|fn_arg| {
                            if let syn::FnArg::Typed(syn::PatType { pat, .. }) = fn_arg {
                                Some(pat)
                            } else {
                                None
                            }
                        })
                        .filter_map(|pat| {
                            if let syn::Pat::Ident(ident) = pat.as_ref() {
                                Some(ident.ident.clone())
                            } else {
                                None
                            }
                        })
                        .collect();

                    let windows_os = "windows";
                    let unix_os = "unix";

                    func_struct_def.push(
                        quote!{
                            #[cfg(target_os = #windows_os)]
                            #def_ident: Option<libloading::os::windows::Symbol<unsafe extern #abi_name fn (#inputs) #output>>,
                            #[cfg(target_os = #unix_os)]
                            #def_ident: Option<libloading::os::unix::Symbol<unsafe extern #abi_name fn (#inputs) #output>>,
                            
                        }
                    );

                    func_struct_new.push(quote! {
                        #def_ident: None,
                    });

                    let error_hint =
                        format!("Unable to load function {:#?} from lib.", ident.to_string());
                    func_struct_init.push(quote! {
                        let #def_ident: libloading::Symbol<unsafe extern #abi_name fn (#inputs) #output> = self.lib.get(#u8str_ident).expect(#error_hint);
                        self.#def_ident = Some(#def_ident.into_raw());
                    });

                    func_struct_impl.push(quote! {
                        pub unsafe fn #ident(&self, #inputs) #output {
                            (self.#def_ident.as_ref().unwrap())(#(#fn_call_args,)*)
                        }
                    });
                }
            }
        };
    }

    let struct_def = quote! {
        #tokens

        pub struct DllLoader {
            lib: libloading::Library,
            #(#func_struct_def)*
        }

        impl DllLoader {
            pub unsafe fn new(path: &str) -> Self {
                let lib = libloading::Library::new(path).unwrap();
                let mut loader = DllLoader {
                    lib,
                    #(#func_struct_new)*
                };
                loader.init();
                loader
            }

            unsafe fn init(& mut self) {
                #(#func_struct_init)*
            }

            #(#func_struct_impl)*
        }
    };

    TokenStream::from(struct_def)
}

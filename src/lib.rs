extern crate proc_macro;
use proc_macro::TokenStream;
use proc_macro2::TokenTree;
use quote::quote;
use syn::{parse_macro_input, spanned::Spanned, visit_mut::VisitMut};
use convert_case::{Case, Casing};
use std::collections::{HashMap, HashSet};

struct IdentRenamer<'dict> {
    ident_dict: &'dict HashMap<String, String>,
    struct_dict: &'dict HashMap<String, HashMap<String, String>>
}

impl<'dict> syn::visit_mut::VisitMut for IdentRenamer<'dict> {
    fn visit_ident_mut(&mut self, ident: &mut syn::Ident) {
        if let Some(new_name) = self.ident_dict.get(&ident.to_string()) {
            *ident = syn::Ident::new(new_name, ident.span());
        }
    }

    fn visit_lit_mut(&mut self, lit: &mut syn::Lit) {
        if let syn::Lit::Str(lit_str) = lit {
            let value = lit_str.value();
            for (old_ident, new_ident) in self.ident_dict {
                if value.contains(old_ident) {
                    let new_value = value.replace(old_ident, new_ident);
                    *lit_str = syn::LitStr::new(&new_value, lit_str.span());
                }
            }
            for (struct_name, struct_fields_dict) in self.struct_dict {
                for (old_field_name, new_field_name) in struct_fields_dict {
                    let old_name = format!("{}::{}", struct_name, old_field_name);
                    let new_name = format!("{}::{}", struct_name, new_field_name);
                    if value.contains(&old_name) {
                        let new_value = value.replace(&old_name, &new_name);
                        *lit_str = syn::LitStr::new(&new_value, lit_str.span());
                    }
                }
            }
        }
    }

    fn visit_macro_mut(&mut self, mac: &mut syn::Macro) {
        let mut new_tokens = proc_macro2::TokenStream::new();
        let mut struct_set: HashSet<String> = HashSet::new();
        for token in mac.tokens.clone() {
            match token {
                TokenTree::Ident(mut ident) => {
                    if let Some(new_name) = self.ident_dict.get(&ident.to_string()) {
                        struct_set.insert(ident.to_string());
                        ident = syn::Ident::new(new_name, ident.span());
                    } else {
                        for struct_name in &struct_set {
                            if self.struct_dict.contains_key(struct_name) {
                                for (old_field_name, new_field_name) in self.struct_dict.get(struct_name).unwrap() {
                                    if &ident.to_string() == old_field_name {
                                        ident = syn::Ident::new(new_field_name, ident.span());
                                    }
                                }
                            }
                        }
                    }
                    new_tokens.extend(Some(TokenTree::Ident(ident)));
                },
                other => new_tokens.extend(Some(other)),
            };
        }
        mac.tokens = new_tokens;
    }

    fn visit_item_impl_mut(&mut self, item_impl: &mut syn::ItemImpl) {
        if let syn::Type::Path(ref mut ty_path) = *item_impl.self_ty {
            let syn::Path{segments, ..} = &mut ty_path.path;
            for segment in segments {
                let ident = &segment.ident;
                if self.struct_dict.contains_key(&ident.to_string()) {
                    let mut impl_field_renamer = ImplFieldRenamer {
                        ident_dict: self.struct_dict.get(&ident.to_string()).unwrap()
                    };
                    for item in &mut item_impl.items {
                        impl_field_renamer.visit_impl_item_mut(item);
                    }
                }
            }
        }
        self.visit_type_mut(&mut item_impl.self_ty);
        self.visit_generics_mut(&mut item_impl.generics);
        for item in &mut item_impl.items {
            self.visit_impl_item_mut(item);
        }
    }
}

struct ImplFieldRenamer<'dict> {
    ident_dict: &'dict HashMap<String, String>
}

impl<'dict> VisitMut for ImplFieldRenamer<'dict> {
    fn visit_member_mut(&mut self, member: &mut syn::Member) {
        if let syn::Member::Named(ident) = member {
            if let Some(new_name) = self.ident_dict.get(&ident.to_string()) {
                *ident = syn::Ident::new(new_name, ident.span());
            }
        }
    }
}

#[proc_macro]
pub fn generate_dll_loader(input: TokenStream) -> TokenStream {

    let input = parse_macro_input!(input as syn::LitStr);
    let input = input.value();

    let bindings = bindgen::Builder::default()
        .header(input)
        .generate()
        .expect("Failed to generate bindings.");

    let mut tokens = syn::parse_str::<syn::File>(&bindings.to_string()).unwrap();

    {
        let test = r"
        struct A {
            a: int,
            b: int
        }
        impl A {
            fn a(&self) {
              let c = self.a + self.b;
            }
        }
        ";
        println!("{:#?}", syn::parse_str::<syn::File>(test));
    }

    let mut func_struct_def = Vec::new();
    let mut func_struct_impl = Vec::new();
    let mut func_struct_new = Vec::new();

    let mut ident_dict: HashMap<String, String> = HashMap::new();
    let mut struct_dict: HashMap<String, HashMap<String, String>> = HashMap::new();

    for item in tokens.items.iter_mut() {
        match item {
            syn::Item::ForeignMod(foreign_mod) => {
                let abi = &foreign_mod.abi;
                let abi_name = abi.name.clone().unwrap();
                for item in foreign_mod.items.iter() {
                    if let syn::ForeignItem::Fn(foreign_fn) = item {
                        let ident = &foreign_fn.sig.ident;
                        let inputs = &foreign_fn.sig.inputs;
                        let output = &foreign_fn.sig.output;

                        // function names to snake case
                        let fn_ident_str = ident.to_string().clone();
                        let fn_ident_converted_str = fn_ident_str.to_case(Case::Snake);
                        let def_ident = syn::Ident::new(&format!("addr_of_{}", &fn_ident_converted_str), ident.span());
                        ident_dict.insert(fn_ident_str, fn_ident_converted_str);

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
                        let unix_os = "linux";

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

                        func_struct_impl.push(quote! {
                            pub unsafe fn #ident(&mut self, #inputs) #output {
                                if self.#def_ident.is_none() {
                                    let #def_ident: libloading::Symbol<unsafe extern #abi_name fn (#inputs) #output> = self.lib.get(#u8str_ident).expect(#error_hint);
                                    self.#def_ident = Some(#def_ident.into_raw());
                                }
                                (self.#def_ident.as_ref().unwrap())(#(#fn_call_args,)*)
                            }
                        });
                    }
                }
            },
            syn::Item::Const(const_value) => {
                let ident_name = const_value.ident.to_string().clone();
                let converted_name = ident_name.to_ascii_uppercase();
                ident_dict.insert(ident_name, converted_name);
            },
            syn::Item::Struct(ref mut struct_node) => {
                let ident_name = struct_node.ident.to_string().clone();
                let converted_name = ident_name.to_case(Case::UpperCamel);
                ident_dict.insert(ident_name.clone(), converted_name);
                struct_dict.insert(ident_name.clone(), HashMap::new());

                for field in &mut struct_node.fields {
                    if let Some(ident) = &mut field.ident {
                        let field_ident_name = ident.to_string();
                        let converted_name = field_ident_name.to_case(Case::Snake);
                        field.ident = Some(syn::Ident::new(&converted_name, ident.span()));
                        struct_dict.get_mut(&ident_name).unwrap().insert(field_ident_name, converted_name);
                    }
                }
            },
            syn::Item::Type(type_def) => {
                let ident_name = type_def.ident.to_string().clone();
                let converted_name = ident_name.to_case(Case::UpperCamel);
                ident_dict.insert(ident_name, converted_name);
            },
            _ => {
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
                DllLoader {
                    lib,
                    #(#func_struct_new)*
                }
            }

            #(#func_struct_impl)*
        }
    };

    // After generating parsed codes, rename symbols we have changed name.
    let mut ident_renamer = IdentRenamer {
        ident_dict: &ident_dict,
        struct_dict: &struct_dict
    };


    let mut struct_def = syn::parse2(struct_def).unwrap();
    ident_renamer.visit_file_mut(&mut struct_def);

    TokenStream::from(quote!(#struct_def))
}

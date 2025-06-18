use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Fields, Ident, ItemEnum};

type TokenStream2 = proc_macro2::TokenStream;

#[proc_macro]
pub fn block_macro(input: TokenStream) -> TokenStream {
    let mut input_enum = parse_macro_input!(input as ItemEnum);
    let enum_name = &input_enum.ident;

    let build_get = build_get_blockstate_id(enum_name, &input_enum);
    let build_from = build_from_blockstate_id(enum_name, &input_enum);
    let build_rotate = build_rotate(enum_name, &input_enum);
    let build_is_rotatable = build_is_rotatable(enum_name, &input_enum);

    let expanded = quote! {
        use crate::server::block::metadata::BlockMetadata;

        #input_enum

        impl #enum_name {
            pub fn get_blockstate_id(&self) -> u16 {
                match self {
                    #(#build_get),*
                }
            }
            pub fn from_blockstate_id(v: u16) -> Self {
                let index = v >> 4;
                match index {
                    #(#build_from),*
                }
            }
            // pub fn rotate(&self) {
            //     match self {
            //         #(#build_rotate),*
            //     }
            // }
            pub fn is_rotatable(&self) -> bool {
                match self {
                    #(#build_is_rotatable),*
                }
            }
        }
        
        // impl From<u16> for #enum_name {
        //     fn from(v: u16) -> Self {
        //         match v {
        //             #(#build_from),*
        //         }
        //     }
        // }
    };
    

    TokenStream::from(expanded)
}

// TODO: Make it iterate only once over all things

fn build_from_blockstate_id(enum_name: &Ident, data_enum: &ItemEnum) -> Vec<TokenStream2> {
    data_enum.variants.iter().enumerate().map(|(block_id, variant)| {
        let variant_ident = &variant.ident;
        let id_lit = syn::LitInt::new(&block_id.to_string(), proc_macro2::Span::call_site());

        match &variant.fields {
            Fields::Named(fields_named) => {
                let field_snippets: Vec<TokenStream2> = fields_named.named.iter().map(|f| {
                    let name = f.ident.as_ref().unwrap();
                    let ty = if let syn::Type::Path(p) = &f.ty {
                        p.path.segments.last().unwrap().ident.to_string()
                    } else {
                        panic!("unsupported field type");
                    };

                    match ty.as_str() {
                        "bool" => quote! {
                            let #name = ((meta >> offset) & 0x01) != 0;
                            offset += 1;
                        },
                        "u8" => quote! {
                            let #name = ((meta >> offset) & 0x0F) as u8;
                            offset += 4;
                        },
                        other => {
                            let ty_ident = syn::Ident::new(other, proc_macro2::Span::call_site());
                            quote! {
                                let #name = #ty_ident::from_meta(((meta >> offset) & ((1 << #ty_ident::meta_size()) - 1)) as u8);
                                offset += #ty_ident::meta_size();
                            }
                        }
                    }
                }).collect();

                let field_names: Vec<_> =
                    fields_named.named.iter().map(|f| f.ident.as_ref().unwrap()).collect();

                quote! {
                    #id_lit => {
                        let meta = (v & 0x0F) as u8;
                        let mut offset: u8 = 0;
                        #( #field_snippets )*
                        #enum_name::#variant_ident { #(#field_names),* }
                    }
                }
            }

            Fields::Unit => quote! {
                #id_lit => #enum_name::#variant_ident
            },

            _ => panic!("unsupported variant form"),
        }
    })
        .chain(std::iter::once(quote! { _ => Blocks::Air }))
        .collect()
}


fn build_get_blockstate_id(enum_name: &Ident, item_enum: &ItemEnum) -> Vec<TokenStream2> {
    item_enum.variants.iter().enumerate().map(|(block_id, variant)| {
        let variant_name = &variant.ident;
        match &variant.fields {
            Fields::Named(fields_named) => {
                let field_idents: Vec<_> = fields_named.named.iter()
                    .map(|f| f.ident.as_ref().unwrap())
                    .collect();

                let field_rotate_calls = fields_named.named.iter().map(|f| {
                    let ident = f.ident.as_ref().unwrap();
                    if let syn::Type::Path(type_path) = &f.ty {

                        let type_indent = type_path.path.get_ident().unwrap();

                        match type_indent.to_string().as_str() {
                            "u8" => quote! {
                                meta |= (#ident & 0x0F) << offset;
                                offset += 4;
                            },
                            "bool" => quote! {
                                meta |= (u8::from(*#ident)) << offset;
                                offset += 1;
                            },
                            _ => quote! {
                                meta |= #ident.get_meta() << offset;
                                offset += #type_indent::meta_size();
                            }
                        }
                    } else {
                        quote! {
                            compile_error!("Unsupported metadata field syntax");
                        }
                    }
                });

                quote! {
                    #enum_name::#variant_name { #( #field_idents ),* } => {
                        let mut meta: u8 = 0;
                        let mut offset: u8 = 0;
                        #( #field_rotate_calls )*
                        ((#block_id as u16) << 4) | (meta as u16)
                    }
                }
            }
            _ => quote! {
                #enum_name::#variant_name => ((#block_id as u16) << 4)
            }
        }
    }).collect()
}

fn build_is_rotatable(enum_name: &Ident, item_enum: &ItemEnum) -> Vec<TokenStream2> {
    item_enum.variants.iter().map(|variant| {
        let variant_name = &variant.ident;

        let is_rotatable = match &variant.fields {
            Fields::Named(fields) => fields.named.iter().any(|f| {
                matches!(&f.ty, syn::Type::Path(type_path) if type_path.path.is_ident("Direction"))
            }),
            _ => false,
        };

        let pattern = match &variant.fields {
            Fields::Named(_) => quote! { #enum_name::#variant_name { .. } },
            _ => quote! { #enum_name::#variant_name }
        };

        quote! {
            #pattern => #is_rotatable
        }
    }).collect()
}


fn build_rotate(enum_name: &Ident, item_enum: &ItemEnum) -> Vec<TokenStream2> {
    item_enum.variants.iter().map(|variant| {
        let variant_name = &variant.ident;
        match &variant.fields {
            Fields::Named(fields_named) => {
                let field_idents: Vec<_> = fields_named.named.iter()
                    .map(|f| f.ident.as_ref().unwrap())
                    .collect();

                let field_rotate_calls = fields_named.named.iter().map(|f| {
                    let ident = f.ident.as_ref().unwrap();
                    if let syn::Type::Path(type_path) = &f.ty {
                        if type_path.path.is_ident("Direction") {
                            quote! {
                                #ident.rotate();
                            }
                        } else {
                            quote! {}
                        }
                    } else {
                        quote! {}
                    }
                });

                quote! {
                    #enum_name::#variant_name { #( #field_idents ),* } => {
                        #( #field_rotate_calls )*
                    }
                }
            }
            _ => quote! {
                #enum_name::#variant_name => {}
            }
        }
    }).collect()
}




use super::key::KeySegment;

pub(crate) fn config_data(input: syn::ItemStruct) -> proc_macro2::TokenStream {
    let lifetime = syn::Lifetime::new("'__c", proc_macro2::Span::call_site());
    let generic = syn::Ident::new("__R", proc_macro2::Span::call_site());
    let ident = input.ident;
    let struct_generics = &input.generics;
    let struct_generics_params: Vec<_> = input.generics.params.iter().collect();
    match input.fields {
        syn::Fields::Named(fields_named) => {
            let (names, types): (Vec<_>, Vec<_>) = fields_named
                .named
                .iter()
                .map(|field| (field.ident.as_ref().unwrap(), &field.ty))
                .unzip();
            let key_segment_type_ts: Vec<_> = names
                .iter()
                .map(|name| KeySegment::name_type_ts(name.to_string().as_str()))
                .collect();
            quote::quote! {
                impl<#lifetime, #(#struct_generics_params,)* #generic>
                    ::inline_config::__private::ConvertFrom<#lifetime, #generic> for #ident #struct_generics
                where
                    #(
                        #generic: ::inline_config::__private::Select<#lifetime, #key_segment_type_ts>,
                        <#generic as ::inline_config::__private::Select<#lifetime, #key_segment_type_ts>>::Representation:
                            ::inline_config::__private::ConvertInto<#lifetime, #types>,
                    )*
                {
                    fn from(representation: &#lifetime #generic) -> #ident #struct_generics {
                        #ident {
                            #(#names:
                                <
                                    <#generic as ::inline_config::__private::Select<#lifetime, #key_segment_type_ts>>::Representation
                                        as ::inline_config::__private::ConvertInto<#lifetime, #types>
                                >::into(
                                    <#generic as ::inline_config::__private::Select<#lifetime, #key_segment_type_ts>>::select(
                                        representation, <#key_segment_type_ts>::default()
                                    )
                                ),
                            )*
                        }
                    }
                }
            }
        }
        syn::Fields::Unnamed(fields_unnamed) => {
            let types: Vec<_> = fields_unnamed
                .unnamed
                .iter()
                .map(|field| &field.ty)
                .collect();
            let key_segment_type_ts: Vec<_> = types
                .iter()
                .enumerate()
                .map(|(index, _)| KeySegment::index_type_ts(index))
                .collect();
            quote::quote! {
                impl<#lifetime, #(#struct_generics_params,)* #generic>
                    ::inline_config::__private::ConvertFrom<#lifetime, #generic> for #ident #struct_generics
                where
                    #(
                        #generic: ::inline_config::__private::Select<#lifetime, #key_segment_type_ts>,
                        <#generic as ::inline_config::__private::Select<#lifetime, #key_segment_type_ts>>::Representation:
                            ::inline_config::__private::ConvertInto<#lifetime, #types>,
                    )*
                {
                    fn from(representation: &#lifetime #generic) -> #ident #struct_generics {
                        #ident(
                            #(
                                <
                                    <#generic as ::inline_config::__private::Select<#lifetime, #key_segment_type_ts>>::Representation
                                        as ::inline_config::__private::ConvertInto<#lifetime, #types>
                                >::into(
                                    <#generic as ::inline_config::__private::Select<#lifetime, #key_segment_type_ts>>::select(
                                        representation, <#key_segment_type_ts>::default()
                                    )
                                ),
                            )*
                        )
                    }
                }
            }
        }
        syn::Fields::Unit => {
            quote::quote! {
                impl<#lifetime, #(#struct_generics_params,)* #generic>
                    ::inline_config::__private::ConvertFrom<#lifetime, #generic> for #ident #struct_generics
                {
                    fn from(representation: &#lifetime #generic) -> #ident #struct_generics {
                        #ident
                    }
                }
            }
        }
    }
}

// struct MyC {
//     a: i64,
//     b: String,
// }

// impl<'c, R> crate::__private::ConvertInto<'c, MyC> for R
// where
//     R: crate::__private::Access<'c, usize>,
//     <R as crate::__private::Access<'c, usize>>::Representation: crate::__private::ConvertInto<'c, i64>,
//     R: crate::__private::Access<'c, ()>,
//     <R as crate::__private::Access<'c, ()>>::Representation: crate::__private::ConvertInto<'c, String>,
// {
//     fn convert(&'c self) -> MyC {
//         MyC {
//             a: self.access(<usize>::default()).convert(),
//             b: self.access(<()>::default()).convert(),
//         }
//     }
// }

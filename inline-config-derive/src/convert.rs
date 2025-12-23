use super::key::KeySegment;

pub(crate) fn select_representation(
    ident: &syn::Ident,
    names: &Option<Vec<String>>,
    fields_ty: &[syn::Type],
) -> Vec<syn::ItemImpl> {
    let names: Vec<_> = match names.as_ref() {
        Some(names) => names.iter().map(Some).collect(),
        None => fields_ty.iter().map(|_| None).collect(),
    };
    names.into_iter().zip(fields_ty).enumerate().flat_map(|(index, (name, field_ty))| {
        let loc = syn::LitInt::from(proc_macro2::Literal::usize_unsuffixed(index));
        let impls: Vec<_> = match name {
            Some(name) => [
                {
                    let key_segment_ty = KeySegment::name_ty(name);
                    syn::parse_quote! {
                        impl<'c> ::inline_config::__private::Select<'c, #key_segment_ty> for #ident {
                            type Representation = #field_ty;

                            fn select(&'c self, _key_segment: #key_segment_ty) -> &'c Self::Representation {
                                &self.#loc
                            }
                        }
                    }
                },
                {
                    let key_segment_ty = KeySegment::index_ty(index);
                    syn::parse_quote! {
                        impl<'c> ::inline_config::__private::Select<'c, #key_segment_ty> for #ident {
                            type Representation = #field_ty;

                            fn select(&'c self, _key_segment: #key_segment_ty) -> &'c Self::Representation {
                                &self.#loc
                            }
                        }
                    }
                },
            ].into(),
            None => [
                {
                    let key_segment_ty = KeySegment::index_ty(index);
                    syn::parse_quote! {
                        impl<'c> ::inline_config::__private::Select<'c, #key_segment_ty> for #ident {
                            type Representation = #field_ty;

                            fn select(&'c self, _key_segment: #key_segment_ty) -> &'c Self::Representation {
                                &self.#loc
                            }
                        }
                    }
                },
            ].into(),
        };
        impls
    }).collect()
}

pub(crate) fn representation_into_containers(
    ident: &syn::Ident,
    names: &Option<Vec<String>>,
    fields_ty: &[syn::Type],
) -> Vec<syn::ItemImpl> {
    let lifetime = syn::Lifetime::new("'__r", proc_macro2::Span::call_site());
    let locs: Vec<_> = fields_ty
        .iter()
        .enumerate()
        .map(|(index, _)| syn::LitInt::from(proc_macro2::Literal::usize_unsuffixed(index)))
        .collect();
    let generics: Vec<_> = fields_ty
        .iter()
        .enumerate()
        .map(|(index, _)| quote::format_ident!("_{index}"))
        .collect();
    if names.is_some() {
        [
            syn::parse_quote! {
                impl<#lifetime, #(#generics),*>
                    ::inline_config::__private::ConvertInto<#lifetime, (#(#generics,)*)> for #ident
                where
                    #(#fields_ty: ::inline_config::__private::ConvertInto<#lifetime, #generics>),*
                {
                    fn into(&#lifetime self) -> (#(#generics,)*) {
                        (
                            #(
                                <#fields_ty as ::inline_config::__private::ConvertInto<#lifetime, #generics>>::into(&self.#locs),
                            )*
                        )
                    }
                }
            },
        ].into()
    } else {
        let generic = syn::Ident::new("__T", proc_macro2::Span::call_site());
        let count = fields_ty.len();
        [
            syn::parse_quote! {
                impl<#lifetime, #(#generics),*>
                    ::inline_config::__private::ConvertInto<#lifetime, (#(#generics,)*)> for #ident
                where
                    #(#fields_ty: ::inline_config::__private::ConvertInto<#lifetime, #generics>),*
                {
                    fn into(&#lifetime self) -> (#(#generics,)*) {
                        (
                            #(
                                <#fields_ty as ::inline_config::__private::ConvertInto<#lifetime, #generics>>::into(&self.#locs),
                            )*
                        )
                    }
                }
            },
            syn::parse_quote! {
                impl<#lifetime, #generic>
                    ::inline_config::__private::ConvertInto<#lifetime, [#generic; #count]> for #ident
                where
                    #(#fields_ty: ::inline_config::__private::ConvertInto<#lifetime, #generic>),*
                {
                    fn into(&#lifetime self) -> [#generic; #count] {
                        [
                            #(
                                <#fields_ty as ::inline_config::__private::ConvertInto<#lifetime, #generic>>::into(&self.#locs),
                            )*
                        ]
                    }
                }
            },
            syn::parse_quote! {
                impl<#lifetime, #generic>
                    ::inline_config::__private::ConvertInto<#lifetime, Vec<#generic>> for #ident
                where
                    #(#fields_ty: ::inline_config::__private::ConvertInto<#lifetime, #generic>),*
                {
                    fn into(&#lifetime self) -> Vec<#generic> {
                        [
                            #(
                                <#fields_ty as ::inline_config::__private::ConvertInto<#lifetime, #generic>>::into(&self.#locs),
                            )*
                        ].into()
                    }
                }
            },
        ].into()
    }
}

fn struct_from_representaiton(
    ident: &syn::Ident,
    struct_generics: &syn::Generics,
    fields: &syn::Fields,
) -> syn::ItemImpl {
    let lifetime = syn::Lifetime::new("'__r", proc_macro2::Span::call_site());
    let generic = syn::Ident::new("__R", proc_macro2::Span::call_site());
    let struct_generics_params = struct_generics.params.iter();
    match fields {
        syn::Fields::Named(fields_named) => {
            let (fields_name, fields_ty): (Vec<_>, Vec<_>) = fields_named
                .named
                .iter()
                .map(|field| (field.ident.as_ref().unwrap(), &field.ty))
                .unzip();
            let key_segment_ty: Vec<_> = fields_name
                .iter()
                .map(|name| KeySegment::name_ty(name.to_string().as_str()))
                .collect();
            syn::parse_quote! {
                impl<#lifetime, #(#struct_generics_params,)* #generic>
                    ::inline_config::__private::ConvertFrom<#lifetime, #generic> for #ident #struct_generics
                where
                    #(
                        #generic: ::inline_config::__private::Select<#lifetime, #key_segment_ty>,
                        <#generic as ::inline_config::__private::Select<#lifetime, #key_segment_ty>>::Representation:
                            ::inline_config::__private::ConvertInto<#lifetime, #fields_ty>,
                    )*
                {
                    fn from(representation: &#lifetime #generic) -> #ident #struct_generics {
                        #ident {
                            #(#fields_name:
                                <
                                    <#generic as ::inline_config::__private::Select<#lifetime, #key_segment_ty>>::Representation
                                        as ::inline_config::__private::ConvertInto<#lifetime, #fields_ty>
                                >::into(
                                    <#generic as ::inline_config::__private::Select<#lifetime, #key_segment_ty>>::select(
                                        representation, <#key_segment_ty>::default()
                                    )
                                ),
                            )*
                        }
                    }
                }
            }
        }
        syn::Fields::Unnamed(fields_unnamed) => {
            let fields_ty: Vec<_> = fields_unnamed
                .unnamed
                .iter()
                .map(|field| &field.ty)
                .collect();
            let key_segment_ty: Vec<_> = fields_ty
                .iter()
                .enumerate()
                .map(|(index, _)| KeySegment::index_ty(index))
                .collect();
            syn::parse_quote! {
                impl<#lifetime, #(#struct_generics_params,)* #generic>
                    ::inline_config::__private::ConvertFrom<#lifetime, #generic> for #ident #struct_generics
                where
                    #(
                        #generic: ::inline_config::__private::Select<#lifetime, #key_segment_ty>,
                        <#generic as ::inline_config::__private::Select<#lifetime, #key_segment_ty>>::Representation:
                            ::inline_config::__private::ConvertInto<#lifetime, #fields_ty>,
                    )*
                {
                    fn from(representation: &#lifetime #generic) -> #ident #struct_generics {
                        #ident(
                            #(
                                <
                                    <#generic as ::inline_config::__private::Select<#lifetime, #key_segment_ty>>::Representation
                                        as ::inline_config::__private::ConvertInto<#lifetime, #fields_ty>
                                >::into(
                                    <#generic as ::inline_config::__private::Select<#lifetime, #key_segment_ty>>::select(
                                        representation, <#key_segment_ty>::default()
                                    )
                                ),
                            )*
                        )
                    }
                }
            }
        }
        syn::Fields::Unit => {
            syn::parse_quote! {
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

pub(crate) fn config_data(input: syn::ItemStruct) -> syn::ItemImpl {
    struct_from_representaiton(&input.ident, &input.generics, &input.fields)
}

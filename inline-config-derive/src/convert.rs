use super::key::KeySegment;

pub(crate) fn config_derive(input: syn::ItemStruct) -> syn::ItemImpl {
    struct_from_representaiton(&input.ident, &input.generics, &input.fields)
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

pub(crate) fn representation_into_containers(
    ident: &syn::Ident,
    fields_name: &[syn::Ident],
    fields_ty: &[syn::Type],
) -> Vec<syn::ItemImpl> {
    let count = fields_name.len();
    let lifetime = syn::Lifetime::new("'__r", proc_macro2::Span::call_site());
    let generic = syn::Ident::new("__T", proc_macro2::Span::call_site());
    [
        syn::parse_quote! {
            impl<#lifetime, #(#fields_name),*>
                ::inline_config::__private::ConvertInto<#lifetime, (#(#fields_name,)*)> for #ident
            where
                #(#fields_ty: ::inline_config::__private::ConvertInto<#lifetime, #fields_name>),*
            {
                fn into(&#lifetime self) -> (#(#fields_name,)*) {
                    (
                        #(
                            <#fields_ty as ::inline_config::__private::ConvertInto<#lifetime, #fields_name>>::into(&self.#fields_name),
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
                            <#fields_ty as ::inline_config::__private::ConvertInto<#lifetime, #generic>>::into(&self.#fields_name),
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
                            <#fields_ty as ::inline_config::__private::ConvertInto<#lifetime, #generic>>::into(&self.#fields_name),
                        )*
                    ].into()
                }
            }
        },
    ].into()
}

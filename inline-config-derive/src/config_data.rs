use super::path::Key;
use darling::FromField;

#[derive(FromField)]
#[darling(attributes(config_data))]
struct ConfigDataFieldAttrs {
    rename: Option<String>,
}

pub(crate) fn config_data(input: syn::ItemStruct) -> syn::ItemImpl {
    let ident = &input.ident;
    let struct_generics = &input.generics;
    let fields = &input.fields;
    let generic = syn::Ident::new("__Repr", proc_macro2::Span::call_site());
    let struct_generics_params = struct_generics.params.iter();
    match fields {
        syn::Fields::Named(fields_named) => {
            let (key_ty, (fields_name, fields_ty)): (Vec<_>, (Vec<_>, Vec<_>)) = fields_named
                .named
                .iter()
                .map(|field| {
                    let ident = field.ident.as_ref().unwrap();
                    let attrs = ConfigDataFieldAttrs::from_field(field)
                        .unwrap_or_else(|e| proc_macro_error::abort_call_site!(e));
                    (
                        Key::name_ty(attrs.rename.unwrap_or(ident.to_string()).as_str()),
                        (ident, &field.ty),
                    )
                })
                .unzip();
            let (access_phantom_generics, convert_phantom_generics): (Vec<_>, Vec<_>) = fields_ty
                .iter()
                .enumerate()
                .map(|(index, _)| {
                    (
                        quote::format_ident!("__AccessPhantom_{index}"),
                        quote::format_ident!("__ConvertPhantom_{index}"),
                    )
                })
                .unzip();
            syn::parse_quote! {
                impl<
                    #(#struct_generics_params,)*
                    #(#access_phantom_generics,)*
                    #(#convert_phantom_generics,)*
                    #generic,
                >
                    ::inline_config::__private::convert::Convert<
                        #generic,
                        (#((#access_phantom_generics, #convert_phantom_generics),)*),
                    > for #ident #struct_generics
                where
                    #(
                        #generic: ::inline_config::__private::key::AccessKey<
                            #key_ty,
                            #access_phantom_generics,
                        >,
                        #fields_ty: ::inline_config::__private::convert::Convert<
                            <#generic as ::inline_config::__private::key::AccessKey<
                                #key_ty,
                                #access_phantom_generics,
                            >>::Repr,
                            #convert_phantom_generics,
                        >,
                    )*
                {
                    fn convert(source: &#generic) -> Self {
                        #ident {
                            #(
                                #fields_name: <#fields_ty as ::inline_config::__private::convert::Convert<
                                    <#generic as ::inline_config::__private::key::AccessKey<
                                        #key_ty,
                                        #access_phantom_generics,
                                    >>::Repr,
                                    #convert_phantom_generics,
                                >>::convert(
                                    <#generic as ::inline_config::__private::key::AccessKey<
                                        #key_ty,
                                        #access_phantom_generics,
                                    >>::access_key(source)
                                ),
                            )*
                        }
                    }
                }
            }
        }
        syn::Fields::Unnamed(fields_unnamed) => {
            let (key_ty, fields_ty): (Vec<_>, Vec<_>) = fields_unnamed
                .unnamed
                .iter()
                .enumerate()
                .map(|(index, field)| (Key::index_ty(index), &field.ty))
                .unzip();
            let (access_phantom_generics, convert_phantom_generics): (Vec<_>, Vec<_>) = fields_ty
                .iter()
                .enumerate()
                .map(|(index, _)| {
                    (
                        quote::format_ident!("__AccessPhantom_{index}"),
                        quote::format_ident!("__ConvertPhantom_{index}"),
                    )
                })
                .unzip();
            syn::parse_quote! {
                impl<
                    #(#struct_generics_params,)*
                    #(#access_phantom_generics,)*
                    #(#convert_phantom_generics,)*
                    #generic,
                >
                    ::inline_config::__private::convert::Convert<
                        #generic,
                        (#((#access_phantom_generics, #convert_phantom_generics),)*),
                    > for #ident #struct_generics
                where
                    #(
                        #generic: ::inline_config::__private::key::AccessKey<
                            #key_ty,
                            #access_phantom_generics,
                        >,
                        #fields_ty: ::inline_config::__private::convert::Convert<
                            <#generic as ::inline_config::__private::key::AccessKey<
                                #key_ty,
                                #access_phantom_generics,
                            >>::Repr,
                            #convert_phantom_generics,
                        >,
                    )*
                {
                    fn convert(source: &#generic) -> Self {
                        #ident(
                            #(
                                <#fields_ty as ::inline_config::__private::convert::Convert<
                                    <#generic as ::inline_config::__private::key::AccessKey<
                                        #key_ty,
                                        #access_phantom_generics,
                                    >>::Repr,
                                    #convert_phantom_generics,
                                >>::convert(
                                    <#generic as ::inline_config::__private::key::AccessKey<
                                        #key_ty,
                                        #access_phantom_generics,
                                    >>::access_key(source)
                                ),
                            )*
                        )
                    }
                }
            }
        }
        syn::Fields::Unit => {
            syn::parse_quote! {
                impl<
                    #(#struct_generics_params,)*
                    #generic,
                >
                    ::inline_config::__private::convert::Convert<
                        #generic,
                        (),
                    > for #ident #struct_generics
                {
                    fn convert(source: &#generic) -> Self {
                        #ident
                    }
                }
            }
        }
    }
}

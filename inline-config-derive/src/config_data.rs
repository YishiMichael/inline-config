use crate::path::Key;
use darling::FromField;

#[derive(FromField)]
#[darling(attributes(config_data))]
struct ConfigDataFieldAttrs {
    rename: Option<String>,
}

pub struct ConfigDataImpls {
    convert_from_impl: syn::ItemImpl,
    non_nil_impl: syn::ItemImpl,
    fields_assertion: syn::ItemConst,
}

impl quote::ToTokens for ConfigDataImpls {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.convert_from_impl.to_tokens(tokens);
        self.non_nil_impl.to_tokens(tokens);
        self.fields_assertion.to_tokens(tokens);
    }
}

pub fn config_data(input: syn::ItemStruct) -> ConfigDataImpls {
    let ident = &input.ident;
    let generics_params: Vec<_> = input.generics.params.iter().collect();
    let where_predicates = input
        .generics
        .where_clause
        .as_ref()
        .map(|where_clause| &where_clause.predicates);
    let lifetime = syn::Lifetime::new("'__inline_config__r", proc_macro2::Span::call_site());
    let generic = syn::Ident::new("__inline_config__R", proc_macro2::Span::call_site());

    let (convert_from_impl, tys) = match &input.fields {
        syn::Fields::Unit => (
            syn::parse_quote! {
                impl<#lifetime, #(#generics_params,)* #generic>
                    ::inline_config::__private::ConvertFrom<#lifetime, #generic> for #ident<#(#generics_params),*>
                where
                    #where_predicates
                {
                    fn convert_from(repr: &#lifetime #generic) -> Self {
                        #ident
                    }
                }
            },
            Vec::new(),
        ),

        syn::Fields::Unnamed(fields_unnamed) => {
            let (key_tys, tys): (Vec<_>, Vec<_>) = fields_unnamed
                .unnamed
                .iter()
                .enumerate()
                .map(|(index, field)| (Key::index_ty(index), &field.ty))
                .unzip();
            (
                syn::parse_quote! {
                    impl<#lifetime, #(#generics_params,)* #generic>
                        ::inline_config::__private::ConvertFrom<#lifetime, #generic> for #ident<#(#generics_params),*>
                    where
                        #(
                            #generic: ::inline_config::__private::AccessKey<#key_tys>,
                            <#generic as ::inline_config::__private::AccessKey<#key_tys>>::Repr:
                                ::inline_config::__private::ConvertInto<#lifetime, #tys>,
                        )*
                        #where_predicates
                    {
                        fn convert_from(repr: &#lifetime #generic) -> Self {
                            #ident(
                                #(
                                    <
                                        <#generic as ::inline_config::__private::AccessKey<#key_tys>>::Repr
                                            as ::inline_config::__private::ConvertInto<#lifetime, #tys>
                                    >::convert_into(
                                        <#generic as ::inline_config::__private::AccessKey<#key_tys>>::access_key(repr)
                                    ),
                                )*
                            )
                        }
                    }
                },
                tys,
            )
        }

        syn::Fields::Named(fields_named) => {
            let (members, (key_tys, tys)): (Vec<_>, (Vec<_>, Vec<_>)) = fields_named
                .named
                .iter()
                .map(|field| {
                    let ident = field.ident.as_ref().unwrap();
                    let attrs = ConfigDataFieldAttrs::from_field(field)
                        .unwrap_or_else(|e| proc_macro_error::abort_call_site!(e));
                    let name = attrs.rename.unwrap_or(ident.to_string());
                    (ident, (Key::name_ty(name.as_str()), &field.ty))
                })
                .unzip();
            (
                syn::parse_quote! {
                    impl<#lifetime, #(#generics_params,)* #generic>
                        ::inline_config::__private::ConvertFrom<#lifetime, #generic> for #ident<#(#generics_params),*>
                    where
                        #(
                            #generic: ::inline_config::__private::AccessKey<#key_tys>,
                            <#generic as ::inline_config::__private::AccessKey<#key_tys>>::Repr:
                                ::inline_config::__private::ConvertInto<#lifetime, #tys>,
                        )*
                        #where_predicates
                    {
                        fn convert_from(repr: &#lifetime #generic) -> Self {
                            #ident {
                                #(#members:
                                    <
                                        <#generic as ::inline_config::__private::AccessKey<#key_tys>>::Repr
                                            as ::inline_config::__private::ConvertInto<#lifetime, #tys>
                                    >::convert_into(
                                        <#generic as ::inline_config::__private::AccessKey<#key_tys>>::access_key(repr)
                                    ),
                                )*
                            }
                        }
                    }
                },
                tys,
            )
        }
    };

    let non_nil_impl = syn::parse_quote! {
        impl<#(#generics_params),*> ::inline_config::__private::NonNil for #ident<#(#generics_params),*> where #where_predicates {}
    };
    let fields_assertion: syn::ItemConst = syn::parse_quote! {
        const _: fn() = || {
            const fn _assert_config_data<T: ::inline_config::__private::ConfigData>() {}
            #(_assert_config_data::<#tys>();)*
        };
    };

    ConfigDataImpls {
        convert_from_impl,
        non_nil_impl,
        fields_assertion,
    }
}

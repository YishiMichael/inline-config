use crate::path::Key;
use darling::FromField;

#[derive(FromField)]
#[darling(attributes(config_data))]
struct ConfigDataFieldAttrs {
    rename: Option<String>,
}

pub struct ConfigDataTokenItems {
    convert_from_impl: syn::ItemImpl,
    non_option_impl: syn::ItemImpl,
}

impl quote::ToTokens for ConfigDataTokenItems {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.convert_from_impl.to_tokens(tokens);
        self.non_option_impl.to_tokens(tokens);
    }
}

pub fn config_data(input: syn::ItemStruct) -> syn::Result<ConfigDataTokenItems> {
    let ident = &input.ident;
    let generics_params: Vec<_> = input.generics.params.iter().collect();
    let where_predicates = input
        .generics
        .where_clause
        .as_ref()
        .map(|where_clause| &where_clause.predicates);
    let mut members = Vec::new();
    let mut key_tys = Vec::new();
    let mut tys = Vec::new();
    match &input.fields {
        syn::Fields::Unit => {}
        syn::Fields::Unnamed(fields_unnamed) => {
            for (index, field) in fields_unnamed.unnamed.iter().enumerate() {
                members.push(syn::Member::from(index));
                key_tys.push(Key::index_ty(index));
                tys.push(&field.ty);
            }
        }
        syn::Fields::Named(fields_named) => {
            for field in &fields_named.named {
                let ident = field.ident.as_ref().unwrap().clone();
                let attrs = ConfigDataFieldAttrs::from_field(&field)
                    .map_err(|e| syn::Error::new_spanned(field, e))?;
                let name = attrs
                    .rename
                    .unwrap_or_else(|| syn::ext::IdentExt::unraw(&ident).to_string());
                members.push(syn::Member::from(ident));
                key_tys.push(Key::name_ty(&name));
                tys.push(&field.ty);
            }
        }
    };
    let generic = syn::Ident::new("__inline_config__S", proc_macro2::Span::call_site());
    Ok(ConfigDataTokenItems {
        convert_from_impl: syn::parse_quote! {
            impl<#(#generics_params,)* #generic>
                ::inline_config::__private::ConvertFrom<#generic>
            for
                #ident<#(#generics_params),*>
            where
                #(
                    #generic: ::inline_config::__private::Access<#key_tys>,
                    <#generic as ::inline_config::__private::Access<#key_tys>>::Repr: ::inline_config::__private::ConvertRepr<#tys>,
                )*
                #where_predicates
            {
                fn convert_from(repr: &'static #generic) -> Self {
                    #ident {
                        #(#members:
                            <
                                <#generic as ::inline_config::__private::Access<#key_tys>>::Repr as ::inline_config::__private::ConvertRepr<#tys>
                            >::convert_repr(
                                <#generic as ::inline_config::__private::Access<#key_tys>>::access(
                                    &repr,
                                ),
                            ),
                        )*
                    }
                }
            }
        },
        non_option_impl: syn::parse_quote! {
            impl<#(#generics_params),*> ::inline_config::__private::NonOption for #ident<#(#generics_params),*> where #where_predicates {}
        },
    })
}

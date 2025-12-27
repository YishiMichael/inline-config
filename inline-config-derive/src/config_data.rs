use crate::path::Key;
use darling::FromField;

#[derive(FromField)]
#[darling(attributes(config_data))]
struct ConfigDataFieldAttrs {
    rename: Option<String>,
}

pub struct ConfigData {
    convert_from_impl: syn::ItemImpl,
    non_nil_impl: syn::ItemImpl,
}

impl syn::parse::Parse for ConfigData {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let input: syn::ItemStruct = input.parse()?;
        let convert_from_impl = match &input.fields {
            syn::Fields::Unit => Self::dispatch((), &input),
            syn::Fields::Unnamed(fields_unnamed) => Self::dispatch(
                fields_unnamed
                    .unnamed
                    .iter()
                    .enumerate()
                    .map(|(index, field)| ArraySlot {
                        index,
                        ty: &field.ty,
                    })
                    .collect::<Vec<_>>(),
                &input,
            ),
            syn::Fields::Named(fields_named) => Self::dispatch(
                fields_named
                    .named
                    .iter()
                    .map(|field| {
                        let ident = field.ident.as_ref().unwrap();
                        let attrs = ConfigDataFieldAttrs::from_field(field)
                            .unwrap_or_else(|e| proc_macro_error::abort_call_site!(e));
                        TableSlot {
                            name: attrs.rename.unwrap_or(ident.to_string()),
                            ident,
                            ty: &field.ty,
                        }
                    })
                    .collect::<Vec<_>>(),
                &input,
            ),
        };
        let non_nil_impl = {
            let generics_params = input.generics.params;
            let where_clause = input.generics.where_clause;
            let ident = input.ident;
            syn::parse_quote! {
                impl #generics_params ::inline_config::__private::NonNil for #ident #generics_params #where_clause {}
            }
        };
        Ok(Self {
            convert_from_impl,
            non_nil_impl,
        })
    }
}

impl ConfigData {
    fn dispatch<S>(config_data_structure: S, input: &syn::ItemStruct) -> syn::ItemImpl
    where
        S: ConfigDataStructure,
    {
        let generics_params: Vec<_> = input.generics.params.iter().collect();
        let where_predicates = input
            .generics
            .where_clause
            .as_ref()
            .map(|where_clause| &where_clause.predicates);
        config_data_structure.convert_from_impl(
            &input.ident,
            generics_params.as_slice(),
            where_predicates,
        )
    }
}

impl quote::ToTokens for ConfigData {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.convert_from_impl.to_tokens(tokens);
        self.non_nil_impl.to_tokens(tokens);
    }
}

trait ConfigDataStructure {
    fn convert_from_impl(
        &self,
        ident: &syn::Ident,
        generics_params: &[&syn::GenericParam],
        where_predicates: Option<&syn::punctuated::Punctuated<syn::WherePredicate, syn::Token![,]>>,
    ) -> syn::ItemImpl;
}

struct ArraySlot<'s> {
    index: usize,
    ty: &'s syn::Type,
}

struct TableSlot<'s> {
    name: String,
    ident: &'s syn::Ident,
    ty: &'s syn::Type,
}

impl ConfigDataStructure for () {
    fn convert_from_impl(
        &self,
        ident: &syn::Ident,
        generics_params: &[&syn::GenericParam],
        where_predicates: Option<&syn::punctuated::Punctuated<syn::WherePredicate, syn::Token![,]>>,
    ) -> syn::ItemImpl {
        let lifetime = syn::Lifetime::new("'__inline_config__r", proc_macro2::Span::call_site());
        let generic = syn::Ident::new("__inline_config__R", proc_macro2::Span::call_site());
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
        }
    }
}

impl ConfigDataStructure for Vec<ArraySlot<'_>> {
    fn convert_from_impl(
        &self,
        ident: &syn::Ident,
        generics_params: &[&syn::GenericParam],
        where_predicates: Option<&syn::punctuated::Punctuated<syn::WherePredicate, syn::Token![,]>>,
    ) -> syn::ItemImpl {
        let (key_tys, tys): (Vec<_>, Vec<_>) = self
            .iter()
            .map(|slot| (Key::index_ty(slot.index), slot.ty))
            .unzip();
        let lifetime = syn::Lifetime::new("'__inline_config__r", proc_macro2::Span::call_site());
        let generic = syn::Ident::new("__inline_config__R", proc_macro2::Span::call_site());
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
        }
    }
}

impl ConfigDataStructure for Vec<TableSlot<'_>> {
    fn convert_from_impl(
        &self,
        ident: &syn::Ident,
        generics_params: &[&syn::GenericParam],
        where_predicates: Option<&syn::punctuated::Punctuated<syn::WherePredicate, syn::Token![,]>>,
    ) -> syn::ItemImpl {
        let (members, (key_tys, tys)): (Vec<_>, (Vec<_>, Vec<_>)) = self
            .iter()
            .map(|slot| (slot.ident, (Key::name_ty(slot.name.as_str()), slot.ty)))
            .unzip();
        let lifetime = syn::Lifetime::new("'__inline_config__r", proc_macro2::Span::call_site());
        let generic = syn::Ident::new("__inline_config__R", proc_macro2::Span::call_site());
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
        }
    }
}

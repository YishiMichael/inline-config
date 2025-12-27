use super::impls::{
    ArrayTypedSlot, ConfigDataStructure, ContainerStructure, TableTypedSlot, UnitStructure,
};
use darling::FromField;

#[derive(FromField)]
#[darling(attributes(config_data))]
struct ConfigDataFieldAttrs {
    rename: Option<String>,
}

pub(crate) struct ConfigData {
    convert_from_impl: syn::ItemImpl,
    non_nil_impl: syn::ItemImpl,
}

impl syn::parse::Parse for ConfigData {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let input: syn::ItemStruct = input.parse()?;
        let convert_from_impl = match &input.fields {
            syn::Fields::Unit => Self::dispatch(UnitStructure, &input),
            syn::Fields::Unnamed(fields_unnamed) => Self::dispatch(
                ContainerStructure {
                    slots: fields_unnamed
                        .unnamed
                        .iter()
                        .enumerate()
                        .map(|(index, field)| ArrayTypedSlot {
                            index,
                            ty: &field.ty,
                        })
                        .collect(),
                },
                &input,
            ),
            syn::Fields::Named(fields_named) => Self::dispatch(
                ContainerStructure {
                    slots: fields_named
                        .named
                        .iter()
                        .map(|field| {
                            let ident = field.ident.as_ref().unwrap();
                            let attrs = ConfigDataFieldAttrs::from_field(field)
                                .unwrap_or_else(|e| proc_macro_error::abort_call_site!(e));
                            TableTypedSlot {
                                name: attrs.rename.unwrap_or(ident.to_string()),
                                ident,
                                ty: &field.ty,
                            }
                        })
                        .collect(),
                },
                &input,
            ),
        };
        let non_nil_impl = {
            let generics_params = input.generics.params;
            let where_clause = input.generics.where_clause;
            let ident = input.ident;
            syn::parse_quote! {
                impl #generics_params ::inline_config::__private::convert::NonNil for #ident #generics_params #where_clause {}
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

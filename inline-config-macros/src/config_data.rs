use crate::path::Key;
use darling::FromField;

#[derive(FromField)]
#[darling(attributes(config_data))]
struct ConfigDataFieldAttrs {
    rename: Option<String>,
}

pub fn config_data(input: syn::ItemStruct) -> syn::Result<syn::ItemImpl> {
    let ident = &input.ident;
    let generics_params: Vec<_> = input.generics.params.iter().collect();
    let where_predicates: Vec<_> = input
        .generics
        .where_clause
        .as_ref()
        .map(|where_clause| where_clause.predicates.iter().collect())
        .unwrap_or_default();
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
                let attrs = ConfigDataFieldAttrs::from_field(field)
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
    let generic = syn::Ident::new("__inline_config__R", proc_macro2::Span::call_site());
    Ok(syn::parse_quote! {
        impl<#(#generics_params,)* #generic> ::inline_config::__private::ConvertData<#generic> for #ident<#(#generics_params),*>
        where
            #(
                #generic: ::inline_config::__private::AccessKey<#key_tys>,
                <#generic as ::inline_config::__private::AccessKey<#key_tys>>::Repr: ::inline_config::__private::Convert<#tys>,
            )*
            #(#where_predicates)*
        {
            fn convert_data() -> Self {
                #ident {
                    #(#members: <<#generic as ::inline_config::__private::AccessKey<#key_tys>>::Repr as ::inline_config::__private::Convert<#tys>>::convert(),)*
                }
            }
        }
    })
}

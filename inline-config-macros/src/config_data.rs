use crate::path::Key;
use darling::FromField;

#[derive(FromField)]
#[darling(attributes(config_data))]
struct ConfigDataFieldArgs {
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

    let (members, key_tys, tys) = match &input.fields {
        syn::Fields::Unit => (Vec::new(), Vec::new(), Vec::new()),
        syn::Fields::Unnamed(fields_unnamed) => fields_unnamed.unnamed.iter().enumerate().fold(
            (Vec::new(), Vec::new(), Vec::new()),
            |(mut members, mut key_tys, mut tys), (index, field)| {
                members.push(syn::Member::from(index));
                key_tys.push(Key::index_ty(index));
                tys.push(&field.ty);
                (members, key_tys, tys)
            },
        ),
        syn::Fields::Named(fields_named) => fields_named.named.iter().try_fold(
            (Vec::new(), Vec::new(), Vec::new()),
            |(mut members, mut key_tys, mut tys), field| {
                let ident = field.ident.as_ref().unwrap().clone();
                let args = ConfigDataFieldArgs::from_field(field)?;
                let name = args
                    .rename
                    .unwrap_or_else(|| syn::ext::IdentExt::unraw(&ident).to_string());
                members.push(syn::Member::from(ident));
                key_tys.push(Key::name_ty(&name));
                tys.push(&field.ty);
                Ok::<_, syn::Error>((members, key_tys, tys))
            },
        )?,
    };
    let generic = syn::Ident::new("__inline_config__R", proc_macro2::Span::call_site());
    Ok(syn::parse_quote! {
        impl<#(#generics_params,)* #generic> From<#generic> for #ident<#(#generics_params),*>
        where
            #(#generic: ::std::ops::Index<#key_tys, Output: Default + Into<#tys>>,)*
            #(#where_predicates)*
        {
            fn from(_value: #generic) -> Self {
                #ident {
                    #(#members:
                        <<#generic as ::std::ops::Index<#key_tys>>::Output as Into<#tys>>::into(
                            <<#generic as ::std::ops::Index<#key_tys>>::Output as Default>::default(),
                        ),
                    )*
                }
            }
        }
    })
}

use crate::path::Key;
use darling::FromField;

#[derive(FromField)]
#[darling(attributes(config))]
struct UnnamedFieldArgs {
    index: Option<syn::LitInt>,
}

#[derive(FromField)]
#[darling(attributes(config))]
struct NamedFieldArgs {
    name: Option<syn::LitStr>,
}

pub fn from_config(input: syn::ItemStruct) -> syn::Result<syn::ItemConst> {
    let ident = &input.ident;
    let generics_params: Vec<_> = input.generics.params.iter().collect();
    let where_predicates: Vec<_> = input
        .generics
        .where_clause
        .as_ref()
        .map(|where_clause| where_clause.predicates.iter().collect())
        .unwrap_or_default();

    let (members, keys, tys) = match &input.fields {
        syn::Fields::Unit => (Vec::new(), Vec::new(), Vec::new()),
        syn::Fields::Unnamed(fields_unnamed) => {
            fields_unnamed.unnamed.iter().enumerate().try_fold(
                (Vec::new(), Vec::new(), Vec::new()),
                |(mut members, mut keys, mut tys), (index, field)| {
                    let args = UnnamedFieldArgs::from_field(field)?;
                    let index = args
                        .index
                        .map(|index| index.base10_parse())
                        .transpose()?
                        .unwrap_or(index);
                    members.push(syn::Member::from(index));
                    keys.push(Key::Index(index));
                    tys.push(&field.ty);
                    Ok::<_, syn::Error>((members, keys, tys))
                },
            )?
        }
        syn::Fields::Named(fields_named) => fields_named.named.iter().try_fold(
            (Vec::new(), Vec::new(), Vec::new()),
            |(mut members, mut keys, mut tys), field| {
                let ident = field.ident.as_ref().unwrap().clone();
                let args = NamedFieldArgs::from_field(field)?;
                let name = args
                    .name
                    .map(|name| name.value())
                    .unwrap_or_else(|| syn::ext::IdentExt::unraw(&ident).to_string());
                members.push(syn::Member::from(ident));
                keys.push(Key::Name(name));
                tys.push(&field.ty);
                Ok::<_, syn::Error>((members, keys, tys))
            },
        )?,
    };
    Ok(syn::parse_quote! {
        const _: () = {
            impl<#(#generics_params,)* __inline_config__T> From<__inline_config__T> for #ident<#(#generics_params),*>
            where
                #(__inline_config__T: ::std::ops::Index<#keys, Output: Default + Into<#tys>>,)*
                #(#where_predicates)*
            {
                fn from(_value: __inline_config__T) -> Self {
                    #ident {
                        #(#members:
                            <<__inline_config__T as ::std::ops::Index<#keys>>::Output as Into<#tys>>::into(
                                <<__inline_config__T as ::std::ops::Index<#keys>>::Output as Default>::default(),
                            ),
                        )*
                    }
                }
            }
        };
    })
}

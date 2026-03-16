use crate::path::Key;
use darling::FromField;

#[derive(FromField)]
#[darling(attributes(from_config))]
struct FromConfigFieldArgs {
    rename: Option<String>,
}

pub fn from_config(input: syn::ItemStruct) -> syn::Result<syn::ItemImpl> {
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
        syn::Fields::Unnamed(fields_unnamed) => fields_unnamed.unnamed.iter().enumerate().fold(
            (Vec::new(), Vec::new(), Vec::new()),
            |(mut members, mut keys, mut tys), (index, field)| {
                members.push(syn::Member::from(index));
                keys.push(Key::Index(index));
                tys.push(&field.ty);
                (members, keys, tys)
            },
        ),
        syn::Fields::Named(fields_named) => fields_named.named.iter().try_fold(
            (Vec::new(), Vec::new(), Vec::new()),
            |(mut members, mut keys, mut tys), field| {
                let ident = field.ident.as_ref().unwrap().clone();
                let args = FromConfigFieldArgs::from_field(field)?;
                let name = args
                    .rename
                    .unwrap_or_else(|| syn::ext::IdentExt::unraw(&ident).to_string());
                members.push(syn::Member::from(ident));
                keys.push(Key::Name(name));
                tys.push(&field.ty);
                Ok::<_, syn::Error>((members, keys, tys))
            },
        )?,
    };
    let m_generic = syn::Ident::new("__inline_config__M", proc_macro2::Span::call_site());
    let p_generic = syn::Ident::new("__inline_config__P", proc_macro2::Span::call_site());
    Ok(syn::parse_quote! {
        impl<#(#generics_params,)* #m_generic, #p_generic>
            From<::inline_config::__private::Config<#m_generic, #p_generic>>
        for
            #ident<#(#generics_params),*>
        where
            #(::inline_config::__private::Config<#m_generic, ::inline_config::__private::PathCons<#p_generic, #keys>>: Into<#tys>,)*
            #(#where_predicates)*
        {
            fn from(_value: ::inline_config::__private::Config<#m_generic, #p_generic>) -> Self {
                #ident {
                    #(#members:
                        <
                            ::inline_config::__private::Config<#m_generic, ::inline_config::__private::PathCons<#p_generic, #keys>> as Into<#tys>
                        >::into(
                            ::inline_config::__private::Config(::std::marker::PhantomData),
                        ),
                    )*
                }
            }
        }
    })
}

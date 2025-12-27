use super::path::Key;

pub(crate) trait ConfigReprStructure {
    fn expr(&self, ident: &syn::Ident, exprs: &[syn::Expr]) -> syn::Expr;
    fn struct_item(
        &self,
        ident: &syn::Ident,
        tys: &[syn::Type],
        vis: &syn::Visibility,
    ) -> syn::ItemStruct;
    fn access_key_impls(&self, ident: &syn::Ident, tys: &[syn::Type]) -> Vec<syn::ItemImpl>;
    fn convert_into_impls(&self, ident: &syn::Ident, tys: &[syn::Type]) -> Vec<syn::ItemImpl>;
}

pub(crate) trait ConfigDataStructure {
    fn convert_from_impl(&self, ident: &syn::Ident, generics: &syn::Generics) -> syn::ItemImpl;
}

pub(crate) struct ArraySlot {
    pub(crate) index: usize,
}

pub(crate) struct TableSlot {
    pub(crate) name: String,
    pub(crate) ident: syn::Ident,
}

pub(crate) struct ArrayTypedSlot<'s> {
    pub(crate) index: usize,
    pub(crate) ty: &'s syn::Type,
}

pub(crate) struct TableTypedSlot<'s> {
    pub(crate) name: String,
    pub(crate) ident: &'s syn::Ident,
    pub(crate) ty: &'s syn::Type,
}

pub(crate) struct UnitStructure;

pub(crate) struct ContainerStructure<S> {
    pub(crate) slots: Vec<S>,
}

impl ConfigReprStructure for ContainerStructure<ArraySlot> {
    fn expr(&self, ident: &syn::Ident, exprs: &[syn::Expr]) -> syn::Expr {
        syn::parse_quote! {
            #ident(
                #(#exprs,)*
            )
        }
    }

    fn struct_item(
        &self,
        ident: &syn::Ident,
        tys: &[syn::Type],
        vis: &syn::Visibility,
    ) -> syn::ItemStruct {
        syn::parse_quote! {
            #vis struct #ident(
                #(#tys,)*
            );
        }
    }

    fn access_key_impls(&self, ident: &syn::Ident, tys: &[syn::Type]) -> Vec<syn::ItemImpl> {
        self.slots
            .iter()
            .zip(tys)
            .map(|(slot, ty)| {
                let key_ty = Key::index_ty(slot.index);
                let loc = syn::Index::from(slot.index);
                syn::parse_quote! {
                    impl ::inline_config::__private::key::AccessKey<#key_ty> for #ident {
                        type Repr = #ty;

                        fn access_key(&self) -> &Self::Repr {
                            &self.#loc
                        }
                    }
                }
            })
            .collect()
    }

    fn convert_into_impls(&self, ident: &syn::Ident, tys: &[syn::Type]) -> Vec<syn::ItemImpl> {
        let locs = self.slots.iter().map(|slot| syn::Index::from(slot.index));
        let lifetime = syn::Lifetime::new("'__inline_config__r", proc_macro2::Span::call_site());
        let generic = syn::Ident::new("__inline_config__T", proc_macro2::Span::call_site());
        [
            syn::parse_quote! {
                impl<#lifetime, #generic>
                    ::inline_config::__private::convert::ConvertInto<#lifetime, Vec<#generic>> for #ident
                where
                    #(#tys: ::inline_config::__private::convert::ConvertInto<#lifetime, #generic>),*
                {
                    fn convert_into(&#lifetime self) -> Vec<#generic> {
                        [
                            #(
                                <#tys as ::inline_config::__private::convert::ConvertInto<#lifetime, #generic>>::convert_into(&self.#locs),
                            )*
                        ].into()
                    }
                }
            },
        ].into()
    }
}

impl ConfigReprStructure for ContainerStructure<TableSlot> {
    fn expr(&self, ident: &syn::Ident, exprs: &[syn::Expr]) -> syn::Expr {
        let locs = self.slots.iter().map(|slot| &slot.ident);
        syn::parse_quote! {
            #ident {
                #(#locs: #exprs,)*
            }
        }
    }

    fn struct_item(
        &self,
        ident: &syn::Ident,
        tys: &[syn::Type],
        vis: &syn::Visibility,
    ) -> syn::ItemStruct {
        let locs = self.slots.iter().map(|slot| &slot.ident);
        syn::parse_quote! {
            #vis struct #ident {
                #(#locs: #tys,)*
            }
        }
    }

    fn access_key_impls(&self, ident: &syn::Ident, tys: &[syn::Type]) -> Vec<syn::ItemImpl> {
        self.slots
            .iter()
            .zip(tys)
            .map(|(slot, ty)| {
                let key_ty = Key::name_ty(slot.name.as_str());
                let loc = &slot.ident;
                syn::parse_quote! {
                    impl ::inline_config::__private::key::AccessKey<#key_ty> for #ident {
                        type Repr = #ty;

                        fn access_key(&self) -> &Self::Repr {
                            &self.#loc
                        }
                    }
                }
            })
            .collect()
    }

    fn convert_into_impls(&self, ident: &syn::Ident, tys: &[syn::Type]) -> Vec<syn::ItemImpl> {
        let (names, locs): (Vec<_>, Vec<_>) = self
            .slots
            .iter()
            .map(|slot| (slot.name.as_str(), &slot.ident))
            .unzip();
        let lifetime = syn::Lifetime::new("'__inline_config__r", proc_macro2::Span::call_site());
        let generic = syn::Ident::new("__inline_config__T", proc_macro2::Span::call_site());
        [
            syn::parse_quote! {
                impl<#lifetime, #generic>
                    ::inline_config::__private::convert::ConvertInto<#lifetime, Vec<(&#lifetime str, #generic)>> for #ident
                where
                    #(#tys: ::inline_config::__private::convert::ConvertInto<#lifetime, #generic>),*
                {
                    fn convert_into(&#lifetime self) -> Vec<(&#lifetime str, #generic)> {
                        [
                            #(
                                (
                                    #names,
                                    <#tys as ::inline_config::__private::convert::ConvertInto<#lifetime, #generic>>::convert_into(&self.#locs),
                                ),
                            )*
                        ].into()
                    }
                }
            },
            syn::parse_quote! {
                impl<#lifetime, #generic>
                    ::inline_config::__private::convert::ConvertInto<#lifetime, Vec<(String, #generic)>> for #ident
                where
                    #(#tys: ::inline_config::__private::convert::ConvertInto<#lifetime, #generic>),*
                {
                    fn convert_into(&#lifetime self) -> Vec<(String, #generic)> {
                        [
                            #(
                                (
                                    #names.to_string(),
                                    <#tys as ::inline_config::__private::convert::ConvertInto<#lifetime, #generic>>::convert_into(&self.#locs),
                                ),
                            )*
                        ].into()
                    }
                }
            },
            syn::parse_quote! {
                impl<#lifetime, #generic>
                    ::inline_config::__private::convert::ConvertInto<#lifetime, ::std::collections::HashMap<&#lifetime str, #generic>> for #ident
                where
                    #(#tys: ::inline_config::__private::convert::ConvertInto<#lifetime, #generic>),*
                {
                    fn convert_into(&#lifetime self) -> ::std::collections::HashMap<&#lifetime str, #generic> {
                        [
                            #(
                                (
                                    #names,
                                    <#tys as ::inline_config::__private::convert::ConvertInto<#lifetime, #generic>>::convert_into(&self.#locs),
                                ),
                            )*
                        ].into()
                    }
                }
            },
            syn::parse_quote! {
                impl<#lifetime, #generic>
                    ::inline_config::__private::convert::ConvertInto<#lifetime, ::std::collections::HashMap<String, #generic>> for #ident
                where
                    #(#tys: ::inline_config::__private::convert::ConvertInto<#lifetime, #generic>),*
                {
                    fn convert_into(&#lifetime self) -> ::std::collections::HashMap<String, #generic> {
                        [
                            #(
                                (
                                    #names.to_string(),
                                    <#tys as ::inline_config::__private::convert::ConvertInto<#lifetime, #generic>>::convert_into(&self.#locs),
                                ),
                            )*
                        ].into()
                    }
                }
            },
        ].into()
    }
}

impl ConfigDataStructure for UnitStructure {
    fn convert_from_impl(&self, ident: &syn::Ident, generics: &syn::Generics) -> syn::ItemImpl {
        let generics_params: Vec<_> = generics.params.iter().collect();
        let where_predicates = generics
            .where_clause
            .as_ref()
            .map(|where_clause| &where_clause.predicates);
        let lifetime = syn::Lifetime::new("'__inline_config__r", proc_macro2::Span::call_site());
        let generic = syn::Ident::new("__inline_config__R", proc_macro2::Span::call_site());
        syn::parse_quote! {
            impl<#lifetime, #(#generics_params,)* #generic>
                ::inline_config::__private::convert::ConvertFrom<#lifetime, #generic> for #ident<#(#generics_params),*>
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

impl ConfigDataStructure for ContainerStructure<ArrayTypedSlot<'_>> {
    fn convert_from_impl(&self, ident: &syn::Ident, generics: &syn::Generics) -> syn::ItemImpl {
        let (key_tys, tys): (Vec<_>, Vec<_>) = self
            .slots
            .iter()
            .map(|slot| (Key::index_ty(slot.index), slot.ty))
            .unzip();
        let generics_params: Vec<_> = generics.params.iter().collect();
        let where_predicates = generics
            .where_clause
            .as_ref()
            .map(|where_clause| &where_clause.predicates);
        let lifetime = syn::Lifetime::new("'__inline_config__r", proc_macro2::Span::call_site());
        let generic = syn::Ident::new("__inline_config__R", proc_macro2::Span::call_site());
        syn::parse_quote! {
            impl<#lifetime, #(#generics_params,)* #generic>
                ::inline_config::__private::convert::ConvertFrom<#lifetime, #generic> for #ident<#(#generics_params),*>
            where
                #(
                    #generic: ::inline_config::__private::key::AccessKey<#key_tys>,
                    <#generic as ::inline_config::__private::key::AccessKey<#key_tys>>::Repr:
                        ::inline_config::__private::convert::ConvertInto<#lifetime, #tys>,
                )*
                #where_predicates
            {
                fn convert_from(repr: &#lifetime #generic) -> Self {
                    #ident(
                        #(
                            <
                                <#generic as ::inline_config::__private::key::AccessKey<#key_tys>>::Repr
                                    as ::inline_config::__private::convert::ConvertInto<#lifetime, #tys>
                            >::convert_into(
                                <#generic as ::inline_config::__private::key::AccessKey<#key_tys>>::access_key(repr)
                            ),
                        )*
                    )
                }
            }
        }
    }
}

impl ConfigDataStructure for ContainerStructure<TableTypedSlot<'_>> {
    fn convert_from_impl(&self, ident: &syn::Ident, generics: &syn::Generics) -> syn::ItemImpl {
        let (locs, (key_tys, tys)): (Vec<_>, (Vec<_>, Vec<_>)) = self
            .slots
            .iter()
            .map(|slot| (slot.ident, (Key::name_ty(slot.name.as_str()), slot.ty)))
            .unzip();
        let generics_params: Vec<_> = generics.params.iter().collect();
        let where_predicates = generics
            .where_clause
            .as_ref()
            .map(|where_clause| &where_clause.predicates);
        let lifetime = syn::Lifetime::new("'__inline_config__r", proc_macro2::Span::call_site());
        let generic = syn::Ident::new("__inline_config__R", proc_macro2::Span::call_site());
        syn::parse_quote! {
            impl<#lifetime, #(#generics_params,)* #generic>
                ::inline_config::__private::convert::ConvertFrom<#lifetime, #generic> for #ident<#(#generics_params),*>
            where
                #(
                    #generic: ::inline_config::__private::key::AccessKey<#key_tys>,
                    <#generic as ::inline_config::__private::key::AccessKey<#key_tys>>::Repr:
                        ::inline_config::__private::convert::ConvertInto<#lifetime, #tys>,
                )*
                #where_predicates
            {
                fn convert_from(repr: &#lifetime #generic) -> Self {
                    #ident {
                        #(#locs:
                            <
                                <#generic as ::inline_config::__private::key::AccessKey<#key_tys>>::Repr
                                    as ::inline_config::__private::convert::ConvertInto<#lifetime, #tys>
                            >::convert_into(
                                <#generic as ::inline_config::__private::key::AccessKey<#key_tys>>::access_key(repr)
                            ),
                        )*
                    }
                }
            }
        }
    }
}

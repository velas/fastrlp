use alloc::vec::Vec;
use proc_macro2::TokenStream;
use quote::quote;

struct ImplWithDeLifetime<'a>(&'a syn::Generics);

impl<'a> quote::ToTokens for ImplWithDeLifetime<'a> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        use proc_macro2::Span;
        use syn::{AttrStyle, Attribute, GenericParam, Lifetime, LifetimeParam};
        if self.0.params.is_empty() {
            // 'de lifetime
            <syn::Token![<]>::default().to_tokens(tokens);
            // add 'de lifetime param
            GenericParam::Lifetime(LifetimeParam::new(Lifetime::new("'de", Span::call_site())))
                .to_tokens(tokens);
            <syn::Token![,]>::default().to_tokens(tokens);
            <syn::Token![>]>::default().to_tokens(tokens);
            return;
        }
        pub struct TokensOrDefault<'a, T: 'a>(pub &'a Option<T>);

        impl<'a, T> quote::ToTokens for TokensOrDefault<'a, T>
        where
            T: quote::ToTokens + Default,
        {
            fn to_tokens(&self, tokens: &mut TokenStream) {
                match self.0 {
                    Some(t) => t.to_tokens(tokens),
                    None => T::default().to_tokens(tokens),
                }
            }
        }

        TokensOrDefault(&self.0.lt_token).to_tokens(tokens);
        // add 'de lifetime param
        GenericParam::Lifetime(LifetimeParam::new(Lifetime::new("'de", Span::call_site())))
            .to_tokens(tokens);
        <syn::Token![,]>::default().to_tokens(tokens);

        // Copy and adopted from syn ImplGenerics
        let mut trailing_or_empty = true;
        for param in self.0.params.pairs() {
            if let syn::GenericParam::Lifetime(_) = **param.value() {
                param.to_tokens(tokens);
                trailing_or_empty = param.punct().is_some();
            }
        }
        fn outer_attr<'a>(
            attrs: &'a [Attribute],
        ) -> core::iter::Filter<core::slice::Iter<'a, Attribute>, fn(&&Attribute) -> bool> {
            fn is_outer(attr: &&Attribute) -> bool {
                match attr.style {
                    AttrStyle::Outer => true,
                    AttrStyle::Inner(_) => false,
                }
            }
            attrs.iter().filter(is_outer)
        }

        for param in self.0.params.pairs() {
            if let syn::GenericParam::Lifetime(_) = **param.value() {
                continue;
            }
            if !trailing_or_empty {
                <syn::Token![,]>::default().to_tokens(tokens);
                trailing_or_empty = true;
            }
            match *param.value() {
                syn::GenericParam::Lifetime(_) => unreachable!(),
                syn::GenericParam::Type(param) => {
                    // Leave off the type parameter defaults
                    for token in outer_attr(&param.attrs) {
                        token.to_tokens(tokens);
                    }
                    param.ident.to_tokens(tokens);
                    if !param.bounds.is_empty() {
                        TokensOrDefault(&param.colon_token).to_tokens(tokens);
                        param.bounds.to_tokens(tokens);
                    }
                }
                syn::GenericParam::Const(param) => {
                    // Leave off the const parameter defaults
                    for token in outer_attr(&param.attrs) {
                        token.to_tokens(tokens);
                    }
                    param.const_token.to_tokens(tokens);
                    param.ident.to_tokens(tokens);
                    param.colon_token.to_tokens(tokens);
                    param.ty.to_tokens(tokens);
                }
            }
            param.punct().to_tokens(tokens);
        }

        TokensOrDefault(&self.0.gt_token).to_tokens(tokens);
    }
}

pub fn impl_decodable(ast: &syn::DeriveInput) -> TokenStream {
    let body = if let syn::Data::Struct(s) = &ast.data {
        s
    } else {
        panic!("#[derive(RlpDecodable)] is only defined for structs.");
    };

    let stmts: Vec<_> = body
        .fields
        .iter()
        .enumerate()
        .map(|(i, field)| decodable_field(i, field))
        .collect();
    let name = &ast.ident;

    let (_, ty_generics, where_clause) = ast.generics.split_for_impl();

    let impl_generics = ImplWithDeLifetime(&ast.generics);

    let impl_block = quote! {
        impl #impl_generics fastrlp::Decodable<'de> for #name #ty_generics #where_clause {
            fn decode(mut buf: &mut &[u8]) -> Result<Self, fastrlp::DecodeError> {
                let b = &mut &**buf;
                let rlp_head = fastrlp::Header::decode(b)?;

                if !rlp_head.list {
                    return ::core::result::Result::Err(fastrlp::DecodeError::UnexpectedString);
                }

                let started_len = b.len();
                let this = Self {
                    #(#stmts)*
                };

                let consumed = started_len - b.len();
                if consumed != rlp_head.payload_length {
                    return ::core::result::Result::Err(fastrlp::DecodeError::ListLengthMismatch {
                        expected: rlp_head.payload_length,
                        got: consumed,
                    });
                }

                *buf = *b;

                ::core::result::Result::Ok(this)
            }
        }
    };

    quote! {
        const _: () = {
            extern crate fastrlp;
            #impl_block
        };
    }
}

pub fn impl_decodable_wrapper(ast: &syn::DeriveInput) -> TokenStream {
    let body = if let syn::Data::Struct(s) = &ast.data {
        s
    } else {
        panic!("#[derive(RlpEncodableWrapper)] is only defined for structs.");
    };

    assert_eq!(
        body.fields.iter().count(),
        1,
        "#[derive(RlpEncodableWrapper)] is only defined for structs with one field."
    );

    let name = &ast.ident;
    let (_, ty_generics, where_clause) = ast.generics.split_for_impl();

    let impl_generics = ImplWithDeLifetime(&ast.generics);
    let impl_block = quote! {
        impl #impl_generics fastrlp::Decodable<'de> for #name #ty_generics #where_clause {
            fn decode(buf: &mut &[u8]) -> Result<Self, fastrlp::DecodeError> {
                ::core::result::Result::Ok(Self(fastrlp::Decodable::decode(buf)?))
            }
        }
    };

    quote! {
        const _: () = {
            extern crate fastrlp;
            #impl_block
        };
    }
}

fn decodable_field(index: usize, field: &syn::Field) -> TokenStream {
    let id = if let Some(ident) = &field.ident {
        quote! { #ident }
    } else {
        let index = syn::Index::from(index);
        quote! { #index }
    };

    quote! { #id: fastrlp::Decodable::decode(b)?, }
}

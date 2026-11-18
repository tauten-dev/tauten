//! Procedural macros for tauten. Use them through the `tauten` crate.
//!
//! * `#[newtype]` / `#[refined(|v| predicate)]` on a one-field tuple struct:
//!   the struct is defined in a private module, so even code next to it can
//!   build values only through `new` / `try_new`.
//! * `#[requires(condition)]` / `#[ensures(|ret| condition)]` on a function
//!   returning `Result`: checked on every call, in every build profile; a
//!   violation is returned as an error, never a panic.

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as Tokens};
use quote::{format_ident, quote};
use syn::visit_mut::VisitMut;
use syn::{
    Attribute, Expr, Fields, ItemFn, ItemStruct, ReturnType, Type, Visibility, parse_macro_input,
};

/// A nominal newtype whose field cannot be reached or built directly.
#[proc_macro_attribute]
pub fn newtype(args: TokenStream, item: TokenStream) -> TokenStream {
    if !args.is_empty() {
        return syn::Error::new(
            Span::call_site(),
            "#[newtype] takes no arguments; use #[refined(..)] for a predicate",
        )
        .to_compile_error()
        .into();
    }
    let item = parse_macro_input!(item as ItemStruct);
    expand_wrapper(&item, None)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

/// A refinement type: a newtype whose values satisfy a predicate.
#[proc_macro_attribute]
pub fn refined(args: TokenStream, item: TokenStream) -> TokenStream {
    let predicate = parse_macro_input!(args as Expr);
    let item = parse_macro_input!(item as ItemStruct);
    expand_wrapper(&item, Some(&predicate))
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

/// Derives that cannot construct a value: they only copy, compare, hash or
/// print an existing one. Anything else (`Default`, `Deserialize`,
/// `Arbitrary`, ...) would create a refined value without `try_new`.
const REFINED_DERIVES: &[&str] = &[
    "Clone",
    "Copy",
    "Debug",
    "PartialEq",
    "Eq",
    "PartialOrd",
    "Ord",
    "Hash",
];

/// Attributes allowed on a `#[refined]` struct besides allowlisted derives.
const REFINED_ATTRS: &[&str] = &["doc", "must_use", "repr"];

fn check_refined_attrs(attrs: &[syn::Attribute]) -> syn::Result<()> {
    for attr in attrs {
        let path = attr.path();
        if path.is_ident("derive") {
            let derives = attr.parse_args_with(
                syn::punctuated::Punctuated::<syn::Path, syn::Token![,]>::parse_terminated,
            )?;
            for derive in &derives {
                let last = derive.segments.last().map(|s| s.ident.to_string());
                if !last.is_some_and(|n| REFINED_DERIVES.contains(&n.as_str())) {
                    return Err(syn::Error::new_spanned(
                        derive,
                        "this derive could construct a refined value without checking the predicate; \
                         allowed derives: Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash. \
                         Implement the trait by hand through `try_new`",
                    ));
                }
            }
        } else if !REFINED_ATTRS.iter().any(|a| path.is_ident(a)) {
            return Err(syn::Error::new_spanned(
                attr,
                "attribute not allowed on a #[refined] type: it could construct a value without \
                 checking the predicate (allowed: doc, must_use, repr, and value-preserving derives)",
            ));
        }
    }
    Ok(())
}

fn expand_wrapper(item: &ItemStruct, predicate: Option<&Expr>) -> syn::Result<Tokens> {
    let Fields::Unnamed(fields) = &item.fields else {
        return Err(syn::Error::new_spanned(
            item,
            "expected a tuple struct with one field: `struct Name(T);`",
        ));
    };
    let mut iter = fields.unnamed.iter();
    let (Some(field), None) = (iter.next(), iter.next()) else {
        return Err(syn::Error::new_spanned(
            &item.fields,
            "expected exactly one field",
        ));
    };
    if !matches!(field.vis, Visibility::Inherited) {
        return Err(syn::Error::new_spanned(
            &field.vis,
            "the field must be private: a public field would let callers skip the constructor",
        ));
    }
    if !item.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &item.generics,
            "generic newtypes are not supported yet",
        ));
    }
    if predicate.is_some() {
        check_refined_attrs(&item.attrs)?;
    }
    let name = &item.ident;
    let vis = &item.vis;
    let inner = &field.ty;
    let attrs = &item.attrs;
    let module = format_ident!("__tauten_{}", name.to_string().to_lowercase());
    let constructors = predicate.map_or_else(
        || {
            quote! {
                /// Wrap a value.
                #[must_use]
                pub const fn new(value: #inner) -> Self {
                    Self(value)
                }
            }
        },
        |predicate| {
            let text = quote!(#predicate).to_string();
            let type_name = name.to_string();
            quote! {
                /// Wrap a value if it satisfies the refinement predicate.
                ///
                /// # Errors
                /// Returns [`tauten::RefineError`] if the predicate is false.
                pub fn try_new(value: #inner) -> ::core::result::Result<Self, ::tauten::RefineError> {
                    if ::tauten::contract::holds(&value, #predicate) {
                        ::core::result::Result::Ok(Self(value))
                    } else {
                        ::core::result::Result::Err(::tauten::RefineError::new(#type_name, #text))
                    }
                }
            }
        },
    );
    let conversions = if predicate.is_some() {
        quote! {
            impl ::core::convert::TryFrom<#inner> for #name {
                type Error = ::tauten::RefineError;
                fn try_from(value: #inner) -> ::core::result::Result<Self, Self::Error> {
                    Self::try_new(value)
                }
            }
        }
    } else {
        quote! {}
    };
    Ok(quote! {
        #[doc(hidden)]
        mod #module {
            #[allow(unused_imports, reason = "the field type may refer to items of the parent module")]
            use super::*;

            #(#attrs)*
            pub struct #name(#inner);

            impl #name {
                #constructors

                /// Borrow the wrapped value.
                #[must_use]
                pub const fn get(&self) -> &#inner {
                    &self.0
                }

                /// Unwrap the value.
                #[must_use]
                pub fn into_inner(self) -> #inner {
                    self.0
                }
            }

            impl ::core::convert::AsRef<#inner> for #name {
                fn as_ref(&self) -> &#inner {
                    &self.0
                }
            }

            #conversions
        }
        #vis use #module::#name;
    })
}

/// Precondition, checked before the body runs.
#[proc_macro_attribute]
pub fn requires(args: TokenStream, item: TokenStream) -> TokenStream {
    contract(Contract::Requires, args, item)
}

/// Postcondition on the `Ok` value, checked after the body returns.
/// `old(expr)` snapshots a `Copy` value before the body runs.
#[proc_macro_attribute]
pub fn ensures(args: TokenStream, item: TokenStream) -> TokenStream {
    contract(Contract::Ensures, args, item)
}

#[derive(Clone, Copy)]
enum Contract {
    Requires,
    Ensures,
}

fn contract(kind: Contract, args: TokenStream, item: TokenStream) -> TokenStream {
    let first = parse_macro_input!(args as Expr);
    let item = parse_macro_input!(item as ItemFn);
    expand_contracts(kind, first, item)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

fn contract_kind(attr: &Attribute) -> Option<Contract> {
    let last = attr.path().segments.last()?;
    if last.ident == "requires" {
        Some(Contract::Requires)
    } else if last.ident == "ensures" {
        Some(Contract::Ensures)
    } else {
        None
    }
}

/// Replaces `old(expr)` with a snapshot binding and records the snapshot.
#[derive(Default)]
struct OldSnapshots {
    taken: Vec<(syn::Ident, Expr)>,
}

impl VisitMut for OldSnapshots {
    fn visit_expr_mut(&mut self, expr: &mut Expr) {
        if let Expr::Call(call) = expr
            && let Expr::Path(path) = call.func.as_ref()
            && path.path.is_ident("old")
            && call.args.len() == 1
            && let Some(arg) = call.args.first()
        {
            let ident = format_ident!("__tauten_old_{}", self.taken.len());
            self.taken.push((ident.clone(), arg.clone()));
            *expr = syn::parse_quote!(#ident);
            return;
        }
        syn::visit_mut::visit_expr_mut(self, expr);
    }
}

fn expand_contracts(kind: Contract, first: Expr, mut item: ItemFn) -> syn::Result<Tokens> {
    if let Some(asyncness) = item.sig.asyncness {
        return Err(syn::Error::new_spanned(
            asyncness,
            "contracts on async functions are not supported yet",
        ));
    }
    let ReturnType::Type(_, ret) = &item.sig.output else {
        return Err(syn::Error::new_spanned(
            &item.sig,
            "contracts need a function returning `Result<T, E>` with `E: From<tauten::ContractViolation>`; \
             a violation is returned as an error, never a panic",
        ));
    };
    if !returns_result(ret) {
        return Err(syn::Error::new_spanned(
            ret,
            "contracts need a `Result<T, E>` return type with `E: From<tauten::ContractViolation>`",
        ));
    }
    let ret = ret.clone();
    let mut pre = Vec::new();
    let mut post = Vec::new();
    match kind {
        Contract::Requires => pre.push(first),
        Contract::Ensures => post.push(first),
    }
    let mut kept = Vec::new();
    for attr in core::mem::take(&mut item.attrs) {
        match contract_kind(&attr) {
            Some(Contract::Requires) => pre.push(attr.parse_args::<Expr>()?),
            Some(Contract::Ensures) => post.push(attr.parse_args::<Expr>()?),
            None => kept.push(attr),
        }
    }
    item.attrs = kept;

    let mut snapshots = OldSnapshots::default();
    for expr in &mut post {
        snapshots.visit_expr_mut(expr);
    }
    let function = item.sig.ident.to_string();
    let checks_pre = pre.iter().map(|cond| {
        let text = quote!(#cond).to_string();
        quote! {
            if !(#cond) {
                return ::core::result::Result::Err(::core::convert::From::from(
                    ::tauten::ContractViolation::requires(#function, #text),
                ));
            }
        }
    });
    let takes = snapshots.taken.iter().map(|(ident, expr)| {
        quote! { let #ident = ::tauten::contract::snapshot(&(#expr)); }
    });
    let checks_post = post.iter().map(|cond| {
        let text = quote!(#cond).to_string();
        quote! {
            if !::tauten::contract::holds(__tauten_ok, #cond) {
                return ::core::result::Result::Err(::core::convert::From::from(
                    ::tauten::ContractViolation::ensures(#function, #text),
                ));
            }
        }
    });
    let attrs = &item.attrs;
    let vis = &item.vis;
    let sig = &item.sig;
    let body = &item.block;
    Ok(quote! {
        #(#attrs)*
        #vis #sig {
            #(#checks_pre)*
            #(#takes)*
            #[allow(clippy::redundant_closure_call, reason = "the body runs in a closure so `return` stays local")]
            let __tauten_result: #ret = (|| -> #ret #body)();
            if let ::core::result::Result::Ok(__tauten_ok) = &__tauten_result {
                #(#checks_post)*
            }
            __tauten_result
        }
    })
}

fn returns_result(ty: &Type) -> bool {
    #[expect(
        clippy::wildcard_enum_match_arm,
        reason = "syn::Type is non_exhaustive; every other type is not a Result"
    )]
    match ty {
        Type::Path(path) => path
            .path
            .segments
            .last()
            .is_some_and(|s| s.ident == "Result"),
        Type::Paren(inner) => returns_result(&inner.elem),
        Type::Group(inner) => returns_result(&inner.elem),
        _ => false,
    }
}

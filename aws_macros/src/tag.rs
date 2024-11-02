use proc_macro::TokenStream;
use quote::quote;

#[derive(Debug)]
enum Translator {
    Serde,
    Manual,
}

pub(crate) fn transform(attr: TokenStream, item: TokenStream) -> TokenStream {
    let root = quote! {::aws};

    let expr: syn::Expr = syn::parse(attr).expect("expected expr in macro attribute");

    let syn::Expr::Assign(assign) = expr else {
        panic!("invalid expression in macro attribute")
    };

    match *assign.left {
        syn::Expr::Path(ref exprpath) => {
            let segments = &exprpath.path.segments;
            let (Some(segment), 1) = (segments.first(), segments.len()) else {
                panic!("invalid attribute key")
            };

            assert!(segment.ident == "translate", "invalid attribute key");
        }
        _ => panic!("invalid expression in tag field attribute, left side"),
    }

    let translator = match *assign.right {
        syn::Expr::Path(ref exprpath) => {
            let segments = &exprpath.path.segments;
            let (Some(segment), 1) = (segments.first(), segments.len()) else {
                panic!("invalid attribute key")
            };

            match segment.ident.to_string().as_str() {
                "serde" => Translator::Serde,
                "manual" => Translator::Manual,
                t => panic!("invalid translator {t}"),
            }
        }
        _ => panic!("invalid expression in tag field attribute, left side"),
    };

    let elem = syn::parse_macro_input!(item as syn::Item);

    let name = match elem {
        syn::Item::Struct(ref s) => &s.ident,
        syn::Item::Enum(ref e) => &e.ident,
        _ => panic!("only applicable to structs/enums"),
    };

    let translator = match translator {
        Translator::Serde => quote! {
            impl #root::tags::TranslatableSerde for #name {}

            impl #root::tags::TagValue<#name> for #name {
                type Error = #root::tags::ParseTagValueError;
                type Translator = #root::tags::TranslateSerde;
            }
        },
        Translator::Manual => quote! {
            impl #root::tags::TranslatableManual for #name {}

            impl #root::tags::TagValue<#name> for #name {
                type Error = #root::tags::ParseTagValueError;
                type Translator = #root::tags::TranslateManual;
            }
        },
    };

    quote! {
        #elem
        #translator
    }
    .into()
}

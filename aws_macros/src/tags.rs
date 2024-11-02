use proc_macro::TokenStream;
use quote::quote;

#[derive(Debug)]
struct Input {
    ident: syn::Ident,
    vis: syn::Visibility,
    elements: Vec<Element>,
}

#[derive(Debug)]
enum ElementKind {
    Required,
    Optional,
}

#[derive(Debug)]
struct Element {
    ident: syn::Ident,
    vis: syn::Visibility,
    ty: syn::Path,
    kind: ElementKind,
    name: String,
}

fn parse_type(input: syn::Type) -> (syn::Path, ElementKind) {
    match input {
        syn::Type::Path(ty) => {
            let segments = ty.path.segments.clone();
            let first = segments.first().expect("segments is empty");

            let (ident, optional) = match first.ident.to_string().as_str() {
                "Option" => match first.arguments {
                    // It may not be required to parse this, we could just
                    // extract the inner type and let the compiler beat the
                    // caller up if there is some bullshit happening.
                    syn::PathArguments::AngleBracketed(ref genargs) => {
                        let mut args = genargs.args.clone();
                        match genargs.args.len() {
                            1 => {
                                let ty = args.pop().expect("genargs are empty");
                                let ty = match ty {
                                    syn::punctuated::Pair::Punctuated(node, _punct) => node,
                                    syn::punctuated::Pair::End(node) => node,
                                };
                                match ty {
                                    syn::GenericArgument::Type(ty) => match ty {
                                        syn::Type::Path(ty) => (ty, ElementKind::Optional),
                                        _ => panic!("invalid generic type for Option"),
                                    },

                                    _ => panic!("need simple owned Option generic"),
                                }
                            }
                            _ => panic!("wrong number of Option generic arguments"),
                        }
                    }
                    _ => panic!("invalid Option usage"),
                },
                _ => (ty, ElementKind::Required),
            };

            (ident.path, optional)
        }
        _ => panic!("invalid field type"),
    }
}

fn parse_field_attrs(attrs: &[syn::Attribute]) -> Option<String> {
    match (attrs.first(), attrs.len()) {
        (Some(attr), 1) => {
            assert!(
                attr.style == syn::AttrStyle::Outer,
                "field attribute style needs to be an outer attribute"
            );
            match attr.meta {
                syn::Meta::List(ref meta_list) => {
                    let tag = &meta_list.path;
                    let tag_name = match (tag.segments.first(), tag.segments.len()) {
                        (Some(segment), 1) => segment.ident.to_string(),
                        (_, 0) => return None,
                        _ => panic!("invalid field attribute path"),
                    };
                    assert!(tag_name == "tag", "invalid field attribute path {tag_name}");

                    let expr: syn::Expr = match meta_list.parse_args() {
                        Ok(expr) => expr,
                        Err(e) => panic!("failed parsing tag field attribute: {e}"),
                    };

                    let syn::Expr::Assign(assign) = expr else {
                        panic!("invalid expression in tag field attribute")
                    };

                    match *assign.left {
                        syn::Expr::Path(ref exprpath) => {
                            let segments = &exprpath.path.segments;
                            let (Some(segment), 1) = (segments.first(), segments.len()) else {
                                panic!("invalid tag field attribute key")
                            };

                            assert!(segment.ident == "key", "invalid tag field attribute key");
                        }
                        _ => panic!("invalid expression in tag field attribute, left side"),
                    }

                    match *assign.right {
                        syn::Expr::Lit(ref expr_lit) => match expr_lit.lit {
                            syn::Lit::Str(ref lit_str) => Some(lit_str.value()),
                            _ => panic!("right side of tag field not a string literal"),
                        },
                        _ => panic!("right side of tag field attribute not a literal"),
                    }
                }
                _ => panic!("invalid field attribute"),
            }
        }
        (_, 0) => None,
        _ => panic!("invalid field attributes"),
    }
}

fn parse_fields(input: impl IntoIterator<Item = syn::Field>) -> Vec<Element> {
    let mut elements = Vec::new();
    for field in input {
        let ident = field.ident.expect("tuple structs not supported");
        let vis = field.vis;
        let (ty, kind) = parse_type(field.ty);

        let name = parse_field_attrs(&field.attrs);

        elements.push(Element {
            ident: ident.clone(),
            vis,
            ty,
            kind,
            name: name.unwrap_or_else(|| ident.to_string()),
        });
    }
    elements
}

fn parse_struct(input: syn::ItemStruct) -> Input {
    Input {
        ident: input.ident,
        vis: input.vis,
        elements: match input.fields {
            syn::Fields::Named(fields) => parse_fields(fields.named),
            _ => panic!("invalid fields"),
        },
    }
}

fn build_output(input: Input) -> TokenStream {
    let root = quote! { ::aws };

    let ident = input.ident;
    let vis = input.vis;

    let type_definition = {
        let elements: Vec<proc_macro2::TokenStream> = input
            .elements
            .iter()
            .map(|element| {
                let ident = &element.ident;
                let vis = &element.vis;
                let ty = &element.ty;
                match element.kind {
                    ElementKind::Required => {
                        quote!(
                            #vis #ident: #ty
                        )
                    }
                    ElementKind::Optional => {
                        quote!(
                            #vis #ident: ::std::option::Option<#ty>
                        )
                    }
                }
            })
            .collect();

        quote! {
            #vis struct #ident {
                #(#elements),*
            }
        }
    };

    let impls = {
        let params = input.elements.iter().map(|element| {
            let ident = &element.ident;
            let ty = &element.ty;
            match element.kind {
                ElementKind::Required => quote!(#ident: #ty),
                ElementKind::Optional => quote!(#ident: ::std::option::Option<#ty>),
            }
        });

        let from_fields: Vec<proc_macro2::TokenStream> = input
            .elements
            .iter()
            .map(|element| {
                let ident = &element.ident;
                quote! {#ident: #ident}
            })
            .collect();

        let from_tags_fields: Vec<proc_macro2::TokenStream>= input.elements.iter().map(|element| {
            let ident = &element.ident;
            let ty = &element.ty;
            let tag_name = &element.name;

            let try_convert = quote!{
                let value: ::std::result::Result<#ty, #root::tags::ParseTagsError> = <#ty as #root::tags::TagValue<#ty>>::from_raw_tag(value)
                    .map_err(
                        |e| #root::tags::ParseTagsError::ParseTag(#root::tags::ParseTagError::InvalidTagValue {
                            key,
                            inner: <<#ty as #root::tags::TagValue<#ty>>::Error as Into<#root::tags::ParseTagValueError>>::into(e),
                        }
                    )
                );

                let value = match value {
                    ::std::result::Result::Ok(v) => v,
                    ::std::result::Result::Err(e) => {
                        return Err(e);
                    }
                };

                value
            };

            let transformer = match element.kind {
                ElementKind::Required => {
                    quote! {
                        let value: #root::tags::RawTagValue = value.ok_or_else(|| #root::tags::ParseTagsError::TagNotFound {
                                key: key.clone()
                            })?
                            .clone();

                        let value = {
                             #try_convert
                        };

                        value

                    }
                }
                ElementKind::Optional => {
                    quote! {
                        let value: ::std::option::Option<#ty> = value.map(|value: #root::tags::RawTagValue| {
                            let value = {
                                 #try_convert
                            };
                            Ok(value)
                        }).transpose()?;
                        value
                    }
                }
            };

            quote! {
                #ident: {
                    let key: #root::tags::TagKey = #root::tags::TagKey::new(#tag_name.to_owned());

                    let value: ::std::option::Option<#root::tags::RawTagValue> = tags
                        .as_slice()
                        .iter()
                        .find(|tag| tag.key() == #tag_name)
                        .map(|tag| tag.value()).cloned();

                    let value = {
                         #transformer
                    };

                    value
                }
            }
        }).collect();

        let fields_to_tags: Vec<proc_macro2::TokenStream> = input
            .elements
            .iter()
            .map(|element| {
                let ident = &element.ident;
                let ty= &element.ty;
                let tag_name = &element.name;
                match element.kind {
                    ElementKind::Required => {
                        quote! {
                            {
                                let key = #root::tags::TagKey::new(#tag_name.to_owned());
                                let value: #root::tags::RawTagValue = <#ty as #root::tags::TagValue<#ty>>::into_raw_tag(self.#ident);
                                v.push(#root::tags::RawTag::new(key, value));
                            }
                        }
                    }
                    ElementKind::Optional => {
                        quote! {
                            {
                                match self.#ident {
                                    ::std::option::Option::Some(value) => {
                                        let key = #root::tags::TagKey::new(#tag_name.to_owned());
                                        let value: #root::tags::RawTagValue = <#ty as #root::tags::TagValue<#ty>>::into_raw_tag(value);
                                        v.push(#root::tags::RawTag::new(key, value));
                                    },
                                    ::std::option::Option::None => {
                                        // do not serialize none values
                                    },
                                }
                            }
                        }
                    }
                }
            })
            .collect();

        quote! {
            impl #ident {
                #vis fn from_values(#(#params),*) -> Self {
                    Self {
                        #(#from_fields),*
                    }
                }

                #vis fn from_tags(tags: #root::tags::TagList) -> Result<Self, #root::tags::ParseTagsError> {
                    Ok(Self {
                        #(#from_tags_fields),*
                    })
                }

                #vis fn into_tags(self) -> #root::tags::TagList {
                    let mut v = ::std::vec::Vec::new();
                    {
                        #(#fields_to_tags);*;
                    }
                    #root::tags::TagList::from_vec(v)
                }
            }
        }
    };

    quote! {
        #type_definition
        #impls
    }
    .into()
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "this is the usual signature for proc macros, and the inner function should have the same signature"
)]
pub(crate) fn transform(attr: TokenStream, item: TokenStream) -> TokenStream {
    assert!(
        attr.is_empty(),
        "cannot take any attribute macro attributes"
    );

    let input = syn::parse_macro_input!(item as syn::Item);

    let input = match input {
        syn::Item::Struct(s) => parse_struct(s),
        _ => panic!("only applicable to structs"),
    };

    build_output(input)
}

//! Defines the HTTP-based Rojo UI. It uses ritz for templating, which is like
//! JSX for Rust. Eventually we should probably replace this with a new
//! framework, maybe using JS and client side rendering.
//!
//! These endpoints generally return HTML and SVG.

use std::{borrow::Cow, sync::Arc, time::Duration};

use hyper::{header, Body, Method, Request, Response, StatusCode};
use maplit::hashmap;
use rbx_dom_weak::types::{Ref, Variant};
use ritz::{html, Fragment, HtmlContent, HtmlSelfClosingTag};

use crate::{
    serve_session::ServeSession,
    snapshot::RojoTree,
    web::{
        assets,
        interface::{ErrorResponse, SERVER_VERSION},
        util::json,
    },
};

pub async fn call(serve_session: Arc<ServeSession>, request: Request<Body>) -> Response<Body> {
    let service = UiService::new(serve_session);

    match (request.method(), request.uri().path()) {
        (&Method::GET, "/") => service.handle_home(),
        (&Method::GET, "/logo.png") => service.handle_logo(),
        (&Method::GET, "/icon.png") => service.handle_icon(),
        (&Method::GET, "/show-instances") => service.handle_show_instances(),
        (_method, path) => json(
            ErrorResponse::not_found(format!("Route not found: {}", path)),
            StatusCode::NOT_FOUND,
        ),
    }
}

pub struct UiService {
    serve_session: Arc<ServeSession>,
}

impl UiService {
    pub fn new(serve_session: Arc<ServeSession>) -> Self {
        UiService { serve_session }
    }

    fn handle_logo(&self) -> Response<Body> {
        Response::builder()
            .header(header::CONTENT_TYPE, "image/png")
            .body(Body::from(assets::logo()))
            .unwrap()
    }

    fn handle_icon(&self) -> Response<Body> {
        Response::builder()
            .header(header::CONTENT_TYPE, "image/png")
            .body(Body::from(assets::icon()))
            .unwrap()
    }

    fn handle_home(&self) -> Response<Body> {
        let page = self.normal_page(html! {
            <div class="button-list">
                { Self::button("Rojo Documentation", "https://rojo.space/docs") }
                { Self::button("View instance tree state", "/show-instances") }
            </div>
        });

        Response::builder()
            .header(header::CONTENT_TYPE, "text/html")
            .body(Body::from(format!("<!DOCTYPE html>{}", page)))
            .unwrap()
    }

    fn handle_show_instances(&self) -> Response<Body> {
        let tree = self.serve_session.tree();
        let root_id = tree.get_root_id();

        let page = self.normal_page(html! {
            { Self::instance(&tree, root_id) }
        });

        Response::builder()
            .header(header::CONTENT_TYPE, "text/html")
            .body(Body::from(format!("<!DOCTYPE html>{}", page)))
            .unwrap()
    }

    fn instance(tree: &RojoTree, id: Ref) -> HtmlContent<'_> {
        let instance = tree.get_instance(id).unwrap();
        let children_list: Vec<_> = instance
            .children()
            .iter()
            .copied()
            .map(|id| Self::instance(tree, id))
            .collect();

        let children_container = if children_list.is_empty() {
            HtmlContent::None
        } else {
            let section = ExpandableSection {
                title: "Children",
                class_name: "instance-children",
                id,
                expanded: true,
                content: html! {
                    { Fragment::new(children_list) }
                },
            };

            section.render()
        };

        let mut properties: Vec<_> = instance.properties().iter().collect();
        properties.sort_by_key(|pair| pair.0);

        let property_list: Vec<_> = properties
            .into_iter()
            .map(|(key, value)| {
                html! {
                    <div class="instance-property" title={ Self::display_value(value) }>
                        { key.clone() } ": " { format!("{:?}", value.ty()) }
                    </div>
                }
            })
            .collect();

        let property_container = if property_list.is_empty() {
            HtmlContent::None
        } else {
            let section = ExpandableSection {
                title: "Properties",
                class_name: "instance-properties",
                id,
                expanded: false,
                content: html! {
                    { Fragment::new(property_list) }
                },
            };

            section.render()
        };

        let metadata_container = {
            let metadata = instance.metadata();

            let relevant_paths = if metadata.relevant_paths.is_empty() {
                HtmlContent::None
            } else {
                let list = metadata
                    .relevant_paths
                    .iter()
                    .map(|path| html! { <li>{ format!("{}", path.display()) }</li> });

                html! {
                    <div>
                        "relevant_paths: "
                        <ul class="path-list">{ Fragment::new(list) }</ul>
                    </div>
                }
            };

            let content = html! {
                <>
                    <div>"specified_id: " { format!("{:?}", metadata.specified_id) } </div>
                    <div>"ignore_unknown_instances: " { metadata.ignore_unknown_instances.to_string() }</div>
                    <div>"instigating source: " { format!("{:?}", metadata.instigating_source) }</div>
                    <div>"middleware: " { format!("{:?}", metadata.middleware) }</div>
                    { relevant_paths }
                </>
            };

            let section = ExpandableSection {
                title: "Metadata",
                class_name: "instance-metadata",
                id,
                expanded: false,
                content,
            };

            section.render()
        };

        let class_name_specifier = if instance.name() == instance.class_name() {
            HtmlContent::None
        } else {
            html! {
                <span>
                    " (" { instance.class_name().to_owned() } ")"
                </span>
            }
        };

        html! {
            <div class="instance">
                <label class="instance-title" for={ format!("instance-{:?}", id) } title={ format!("ref: {:?}", instance.id())}>
                    { instance.name().to_owned() }
                    { class_name_specifier }
                </label>
                { metadata_container }
                { property_container }
                { children_container }
            </div>
        }
    }

    fn display_value(value: &Variant) -> String {
        match value {
            Variant::String(value) => value.clone(),
            Variant::Bool(value) => value.to_string(),
            _ => format!("{:?}", value),
        }
    }

    fn stat_item<S: Into<String>>(name: &str, value: S) -> HtmlContent<'_> {
        html! {
            <span class="stat">
                <span class="stat-name">{ name } ": "</span>
                <span class="stat-value">{ value.into() }</span>
            </span>
        }
    }

    fn button<'a>(text: &'a str, href: &'a str) -> HtmlContent<'a> {
        html! {
            <a class="button" href={ href }>{ text }</a>
        }
    }

    fn normal_page<'a>(&'a self, body: HtmlContent<'a>) -> HtmlContent<'a> {
        let project_name = self.serve_session.project_name();
        let uptime = {
            let elapsed = self.serve_session.start_time().elapsed();

            // Round off all of our sub-second precision to make timestamps
            // nicer.
            let just_nanos = Duration::from_nanos(u64::from(elapsed.subsec_nanos()));
            let elapsed = elapsed - just_nanos;

            humantime::format_duration(elapsed).to_string()
        };

        Self::page(html! {
            <div class="root">
                <header class="header">
                    <a class="main-logo" href="/">
                        <img src="/logo.png" />
                    </a>
                    <div class="stats">
                        { Self::stat_item("Server Version", SERVER_VERSION) }
                        { Self::stat_item("Project", project_name) }
                        { Self::stat_item("Server Uptime", uptime) }
                    </div>
                </header>
                <main class="main">
                    { body }
                </main>
            </div>
        })
    }

    fn page(body: HtmlContent<'_>) -> HtmlContent<'_> {
        html! {
            <html>
                <head>
                    <meta charset="utf8" />
                    <title>"Rojo Live Server"</title>
                    <link rel="icon" type="image/png" sizes="32x32" href="/icon.png" />
                    <meta name="viewport" content="width=device-width, initial-scale=1, minimum-scale=1, maximum-scale=1" />
                    <style>
                        { ritz::UnescapedText::new(assets::css()) }
                    </style>
                </head>

                <body>
                    { body }
                </body>
            </html>
        }
    }
}

struct ExpandableSection<'a> {
    title: &'a str,
    class_name: &'a str,
    id: Ref,
    expanded: bool,
    content: HtmlContent<'a>,
}

impl<'a> ExpandableSection<'a> {
    fn render(self) -> HtmlContent<'a> {
        let input_id = format!("{}-{:?}", self.class_name, self.id);

        // We need to specify this input manually because Ritz doesn't have
        // support for conditional attributes like `checked`.
        let mut input = HtmlSelfClosingTag {
            name: Cow::Borrowed("input"),
            attributes: hashmap! {
                Cow::Borrowed("class") => Cow::Borrowed("expandable-input"),
                Cow::Borrowed("id") => Cow::Owned(input_id.clone()),
                Cow::Borrowed("type") => Cow::Borrowed("checkbox"),
            },
        };

        if self.expanded {
            input
                .attributes
                .insert(Cow::Borrowed("checked"), Cow::Borrowed("checked"));
        }

        html! {
            <section class="expandable-section">
                { input }

                <h1 class="expandable-label">
                    <label for={ input_id.clone() }>
                        <span class="expandable-visualizer"></span>
                        { self.title }
                    </label>
                </h1>
                <div class={ format!("expandable-items {}", self.class_name) }>
                    { self.content }
                </div>
            </section>
        }
    }
}

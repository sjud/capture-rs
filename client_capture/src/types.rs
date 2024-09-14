use crate::{timestamp, window, SNAPSHOT_TIME, TIME_OF_LAST_MUTATION};
use js_sys::Array;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use wasm_bindgen::JsCast;
use web_sys::{
    Document, DocumentType, DomException, Element, GetRootNodeOptions, MutationRecord, Node,
    SvgElement,
};

use crate::{
    snapshot::{find_root_id, map_node_to_id},
    NODE_MAP, ROOTS, SERIALIZED_NODE_MAP,
};

#[cfg_attr(feature = "influxdb", derive(influxdb::InfluxDbWriteable))]
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum SerializedNode {
    DocumentNode(DocumentNode),
    ElementNode(ElementNode),
    TextNode(TextNode),
    CommentNode(CommentNode),
    CDataNode(CDataNode),
    DocumentTypeNode(DocumentTypeNode),
}

impl SerializedNode {
    pub fn set_attribute(&mut self, name: String, value: String) {
        match self {
            SerializedNode::ElementNode(this) => {
                let mut attributes = this
                    .attributes
                    .clone()
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|(n, _)| n != &name)
                    .collect::<Vec<(String, String)>>();
                attributes.push((name, value));
            }
            _ => panic!("this node doesn't support attributes"),
        }
    }
    pub fn id(&self) -> u32 {
        match self {
            SerializedNode::DocumentNode(this) => this.id,
            SerializedNode::ElementNode(this) => this.id,
            SerializedNode::TextNode(this) => this.id,
            SerializedNode::CommentNode(this) => this.id,
            SerializedNode::CDataNode(this) => this.id,
            SerializedNode::DocumentTypeNode(this) => this.id,
        }
    }
    pub fn set_text_content(&mut self, text_content: Option<String>) {
        match self {
            SerializedNode::TextNode(this) => this.text_content = text_content,
            SerializedNode::CommentNode(this) => this.text_content = text_content,
            SerializedNode::CDataNode(this) => this.text_content = text_content,
            _ => panic!("this node doesn't have text content"),
        }
    }
    pub fn push_child(&mut self, child: u32) {
        match self {
            SerializedNode::DocumentNode(this) => {
                let mut current_children = this.child_nodes.clone().unwrap_or_default();
                current_children.reverse();
                current_children.push(child);
                current_children.reverse();
                this.child_nodes = Some(current_children);
            }
            SerializedNode::ElementNode(this) => {
                let mut current_children = this.child_nodes.clone().unwrap_or_default();
                current_children.reverse();
                current_children.push(child);
                current_children.reverse();
                this.child_nodes = Some(current_children);
            }
            SerializedNode::TextNode(_) => {}
            SerializedNode::CommentNode(_) => {}
            SerializedNode::CDataNode(_) => {}
            SerializedNode::DocumentTypeNode(_) => {}
        }
    }
    /// Returns a list of child node ids (if any)
    /// This function just parses the values in Node, it doesn't access any global variables of our program
    pub fn build(
        &self,
        parent: &Node,
        prev_sibling: Option<Node>,
        next_sibling: Option<Node>,
    ) -> Result<(Node, Vec<u32>), DomException> {
        match self {
            SerializedNode::ElementNode(ElementNode {
                tag_name,
                attributes,
                child_nodes,
                is_custom,
                ..
            }) => {
                let tag_name = if *is_custom {
                    "div"
                } else if tag_name == "SCRIPT" {
                    "noscript"
                } else {
                    tag_name.as_str()
                };
                let node = window()
                    .document()
                    .unwrap()
                    .create_element(tag_name)
                    .map_err(|err| err.unchecked_into::<DomException>())?
                    .dyn_into::<Node>()
                    .unwrap();
                if let Some(prev_sibling) = prev_sibling {
                    prev_sibling
                        .unchecked_into::<Element>()
                        .after_with_node_1(&node)
                        .map_err(|err| err.unchecked_into::<DomException>())?;
                } else if let Some(next_sibling) = next_sibling {
                    next_sibling
                        .unchecked_into::<Element>()
                        .before_with_node_1(&node)
                        .map_err(|err| err.unchecked_into::<DomException>())?;
                } else {
                    parent
                        .append_child(&node)
                        .map_err(|err| err.unchecked_into::<DomException>())?;
                }
                let el = node.unchecked_ref::<Element>();
                for (name, value) in attributes.as_ref().cloned().unwrap_or_default() {
                    el.set_attribute(name.as_str(), value.as_str())
                        .map_err(|err| err.unchecked_into::<DomException>())?;
                }
                Ok((node, child_nodes.as_ref().cloned().unwrap_or_default()))
            }
            SerializedNode::CommentNode(CommentNode { text_content, .. }) => {
                let text = text_content
                    .as_ref()
                    .map(|s| s.as_str())
                    .unwrap_or_else(|| "");
                let comment = window().document().unwrap().create_comment(text);
                let node = parent
                    .append_child(&comment.unchecked_into::<Node>())
                    .map_err(|err| err.unchecked_into::<DomException>())?;
                Ok((node, Vec::new()))
            }
            SerializedNode::TextNode(TextNode { text_content, .. }) => {
                let text = text_content
                    .as_ref()
                    .map(|s| s.as_str())
                    .unwrap_or_else(|| "");
                let comment = window().document().unwrap().create_text_node(text);
                let node = parent
                    .append_child(&comment.unchecked_into::<Node>())
                    .map_err(|err| err.unchecked_into::<DomException>())?;
                Ok((node, Vec::new()))
            }
            // expect the document node of the iframe to be the parent, we won't append a document node just add its compat node
            SerializedNode::DocumentNode(DocumentNode { child_nodes, .. }) => {
                // not sure what to do here, compat_mode was to fix a bug that I don't know yet
                Ok((
                    parent.clone(),
                    child_nodes.as_ref().cloned().unwrap_or_else(|| Vec::new()),
                ))
            }
            SerializedNode::CDataNode(_) => todo!(),
            SerializedNode::DocumentTypeNode(DocumentTypeNode {
                name,
                public_id,
                system_id,
                ..
            }) => {
                //expect parent to be the document
                let document = parent.unchecked_ref::<Document>();
                let doc_type = document
                    .implementation()
                    .unwrap()
                    .create_document_type(name, public_id, system_id)
                    .map_err(|err| err.unchecked_into::<DomException>())?;
                Ok((doc_type.unchecked_into::<Node>(), Vec::new()))
            }
        }
    }
    pub fn new(node: &Node, id: u32) -> Self {
        let mut is_shadow_host = false;
        let mut is_shadow = false;
        // composed = true means that for a given shadow root, getRoot() will return self.
        let root = node.get_root_node_with_options(&{
            let opt = GetRootNodeOptions::new();
            opt.set_composed(true);
            opt
        });

        // JS node equality is based on memory location.
        if root.loose_eq(node.as_ref()) {
            // If the root doesn't already exist in roots add it.
            if find_root_id(&node).is_none() {
                ROOTS.with(|roots| roots.borrow_mut().push((node.clone(), id)));
                // If the current node type doesn't equal the document node and it is a root, then it is a shadow root
                if node.node_type() != 9 {
                    is_shadow_host = true;
                }
            }
        }

        // If root type doesn't equal document node, then the current node is shadow
        if root.node_type() != 9 {
            is_shadow = true;
        }

        // This panics if a root of a node is not in roots. This could happen if the child is evaluated by this function before the parent.
        // So if the node in the functions argument is not the dom root, and is called before the dom root, or any other child before parent relation.
        // if the root id doesn't exist in our map, assume the id is 0
        let root_id = find_root_id(&node).unwrap_or_default();

        match node.node_type() {
            1 => Self::ElementNode({
                let el = node.unchecked_ref::<Element>();
                ElementNode {
                    id,
                    root_id,
                    is_shadow_host,
                    is_shadow,
                    tag_name: el.tag_name(),
                    // Because the namespace of an attribute is an attribute on a parent and we serialize the entire document including all parents
                    // attributes will be correctly namespaced the same way they would be correctly namespaced a normal document.
                    attributes: {
                        let attributes = el.attributes();
                        let mut list = Vec::new();
                        for i in 0..attributes.length() {
                            let attr = attributes.item(i).unwrap();
                            let value = if attr.name() == "href" {
                                format!(
                                    "{}{}",
                                    {
                                        let mut href = window().location().href().unwrap();
                                        // remove trailing slash
                                        href.pop();
                                        href
                                    },
                                    attr.value()
                                )
                            } else {
                                attr.value()
                            };
                            list.push((attr.name(), value));
                        }
                        Some(list)
                    },
                    child_nodes: None,
                    is_svg: { el.dyn_ref::<SvgElement>().is_some() },
                    need_block: {
                        //TODO
                        false
                    },
                    is_custom: !HTML_TAGS.contains(&el.tag_name().as_str()),
                }
            }),
            3 => Self::TextNode(TextNode {
                id,
                root_id,
                is_shadow_host,
                is_shadow,
                text_content: node.text_content(),
            }),
            9 => Self::DocumentNode(DocumentNode {
                id,
                root_id,
                is_shadow_host,
                is_shadow,
                child_nodes: None,
                compat_mode: node.unchecked_ref::<Document>().compat_mode(),
            }),
            8 => Self::CommentNode(CommentNode {
                id,
                root_id,
                is_shadow_host,
                is_shadow,
                text_content: node.text_content(),
            }),
            10 => Self::DocumentTypeNode({
                let doc_ty = node.unchecked_ref::<DocumentType>();
                DocumentTypeNode {
                    id,
                    root_id,
                    is_shadow_host,
                    is_shadow,
                    name: doc_ty.name(),
                    public_id: doc_ty.public_id(),
                    system_id: doc_ty.system_id(),
                }
            }),
            4 => Self::CDataNode(CDataNode {
                id,
                root_id,
                is_shadow_host,
                is_shadow,
                text_content: node.text_content(),
            }),
            _ => todo!(),
        }
    }
}
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct CDataNode {
    pub id: u32,
    pub root_id: u32,
    pub is_shadow_host: bool,
    pub is_shadow: bool,
    pub text_content: Option<String>,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct DocumentTypeNode {
    pub id: u32,
    pub root_id: u32,
    pub is_shadow_host: bool,
    pub is_shadow: bool,
    pub name: String,
    pub public_id: String,
    pub system_id: String,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct DocumentNode {
    pub id: u32,
    pub root_id: u32,
    pub is_shadow_host: bool,
    pub is_shadow: bool,
    pub child_nodes: Option<Vec<u32>>,
    pub compat_mode: String,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct ElementNode {
    pub id: u32,
    pub root_id: u32,
    pub is_shadow_host: bool,
    pub is_shadow: bool,
    pub tag_name: String,
    pub attributes: Option<Vec<(String, String)>>,
    pub child_nodes: Option<Vec<u32>>,
    pub is_svg: bool,
    pub need_block: bool,
    pub is_custom: bool,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct TextNode {
    pub id: u32,
    pub root_id: u32,
    pub is_shadow_host: bool,
    pub is_shadow: bool,
    pub text_content: Option<String>,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct CommentNode {
    pub id: u32,
    pub root_id: u32,
    pub is_shadow_host: bool,
    pub is_shadow: bool,
    pub text_content: Option<String>,
}

pub struct SnapShot {
    pub node_map: HashMap<u32, Node>,
    pub serialized_node_map: HashMap<u32, SerializedNode>,
    pub roots: Vec<(Node, u32)>,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum MutationVariant {
    ChildListAdded((MutationChildList, HashMap<u32, SerializedNode>)),
    ChildListRemoved(MutationChildList),
    CharacterData(MutationCharacterData),
    Attributes(MutationAttributes),
}
impl MutationVariant {
    pub fn millis(&self) -> f64 {
        match self {
            MutationVariant::ChildListAdded((this, _)) => this.millis,
            MutationVariant::ChildListRemoved(this) => this.millis,
            MutationVariant::CharacterData(this) => this.millis,
            MutationVariant::Attributes(this) => this.millis,
        }
    }
    pub fn target_id(&self) -> u32 {
        match self {
            MutationVariant::ChildListAdded((this, _)) => this.target_id,
            MutationVariant::ChildListRemoved(this) => this.target_id,
            MutationVariant::CharacterData(this) => this.target_id,
            MutationVariant::Attributes(this) => this.target_id,
        }
    }
    pub fn new(record: MutationRecord) -> Self {
        match record.type_().as_str() {
            "attributes" => Self::Attributes(MutationAttributes::new(record)),
            "characterData" => Self::CharacterData(MutationCharacterData::new(record)),
            "childList" => {
                if record.added_nodes().unchecked_into::<Array>().length() != 0 {
                    Self::ChildListAdded(MutationChildList::added(record))
                } else if record.removed_nodes().unchecked_into::<Array>().length() != 0 {
                    Self::ChildListRemoved(MutationChildList::removed(record))
                } else {
                    panic!("expecting child list to always have either added or removed nodes")
                }
            }
            other => panic!("{other} was not a mutation type specified"),
        }
    }
}
/// returns (TargetNode,TargetId)
fn target(record: &MutationRecord) -> (Node, u32) {
    let target = record
        .target()
        .expect("MutationRecord target can't be null?");
    let id = map_node_to_id(&target).expect("target to already exist in the node map");
    (target, id)
}
fn millis() -> f64 {
    TIME_OF_LAST_MUTATION.with(|last_time| {
        let mut ts = timestamp();
        if ts <= *last_time.borrow() {
            ts += 0.0001
        };
        *last_time.borrow_mut() = ts;
        ts
    })
}
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct MutationAttributes {
    pub target_id: u32,
    pub millis: f64,
    pub attribute: Option<(String, String)>,
}
impl MutationAttributes {
    pub fn new(record: MutationRecord) -> Self {
        let (target, target_id) = target(&record);
        let attribute = record.attribute_name().map(|name| {
            let t = (
                target
                    .dyn_ref::<Element>()
                    .expect(
                        "Attribute mutation record to only apply to nodes that are valid Elements",
                    )
                    .get_attribute(&name)
                    .expect("attribute name to have value"),
                name,
            );
            // we ordered it as above (value,name) to use a reference and then consume name. now reorder it so it's (name,value)
            (t.1, t.0)
        });
        Self {
            target_id,
            millis: millis(),
            attribute,
        }
    }
}
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct MutationCharacterData {
    pub target_id: u32,
    pub millis: f64,
    pub text_content: Option<String>,
}
impl MutationCharacterData {
    pub fn new(record: MutationRecord) -> Self {
        let (target, target_id) = target(&record);
        Self {
            target_id,
            millis: millis(),
            text_content: target.text_content(),
        }
    }
}
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct MutationChildList {
    pub target_id: u32,
    pub millis: f64,
    pub prev_sibling: Option<u32>,
    pub next_sibling: Option<u32>,
    pub nodes: Vec<u32>,
}
/// returns (PreviousSiblingId,NextSiblingId)
fn siblings(record: &MutationRecord) -> (Option<u32>, Option<u32>) {
    (
        record
            .previous_sibling()
            .and_then(|node| map_node_to_id(&node)),
        record.next_sibling().and_then(|node| map_node_to_id(&node)),
    )
}

impl MutationChildList {
    pub fn added(record: MutationRecord) -> (Self, HashMap<u32, SerializedNode>) {
        let (target, target_id) = target(&record);
        let (prev_sibling, next_sibling) = siblings(&record);
        let mut added_nodes = Vec::new();
        for node in record.added_nodes().values() {
            let node = node
                .unwrap()
                .dyn_into::<Node>()
                .expect("nodelist to return node");
            added_nodes.push(node);
        }
        let serialized_nodes =
            crate::observer::mutation_parse_added_nodes(added_nodes.clone(), target);
        let nodes = added_nodes
            .iter()
            .map(|node| map_node_to_id(node).expect("node to be in map by now"))
            .collect::<Vec<_>>();
        (
            Self {
                target_id,
                millis: millis(),
                prev_sibling,
                next_sibling,
                nodes,
            },
            serialized_nodes,
        )
    }
    pub fn removed(record: MutationRecord) -> Self {
        let (_, target_id) = target(&record);
        let (prev_sibling, next_sibling) = siblings(&record);
        let mut nodes = Vec::new();
        for node in record.removed_nodes().values() {
            let node = node
                .unwrap()
                .dyn_into::<Node>()
                .expect("nodelist to return node");
            let id = map_node_to_id(&node).expect("removed node to be in node map");
            nodes.push(id);
            NODE_MAP.with(|node_map| node_map.borrow_mut().remove(&id));
            SERIALIZED_NODE_MAP.with(|node_map| node_map.borrow_mut().remove(&id));
            clean_up(&node);
        }
        Self {
            target_id,
            millis: millis(),
            prev_sibling,
            next_sibling,
            nodes,
        }
    }
}

fn clean_up(node: &Node) {
    for child in node.child_nodes().values() {
        let node = child
            .unwrap()
            .dyn_into::<Node>()
            .expect("nodelist to return node");
        let id = map_node_to_id(&node).expect("removed node to be in node map");

        NODE_MAP.with(|node_map| {
            node_map.borrow_mut().remove(&id);
        });

        SERIALIZED_NODE_MAP.with(|node_map| {
            node_map.borrow_mut().remove(&id);
        });

        clean_up(&node);
    }
}

const HTML_TAGS: [&str; 125] = [
    "A",
    "ABBR",
    "ACRONYM",
    "ADDRESS",
    "APPLET",
    "AREA",
    "ARTICLE",
    "ASIDE",
    "AUDIO",
    "B",
    "BASE",
    "BASEFONT",
    "BDI",
    "BDO",
    "BIG",
    "BLOCKQUOTE",
    "BODY",
    "BR",
    "BUTTON",
    "CANVAS",
    "CAPTION",
    "CENTER",
    "CITE",
    "CODE",
    "COL",
    "COLGROUP",
    "DATA",
    "DATALIST",
    "DD",
    "DEL",
    "DETAILS",
    "DFN",
    "DIALOG",
    "DIR",
    "DIV",
    "DL",
    "DT",
    "EM",
    "EMBED",
    "FIELDSET",
    "FIGCAPTION",
    "FIGURE",
    "FONT",
    "FOOTER",
    "FORM",
    "FRAME",
    "FRAMESET",
    "H1",
    "H2",
    "H3",
    "H4",
    "H5",
    "H6",
    "HEAD",
    "HEADER",
    "HGROUP",
    "HR",
    "HTML",
    "I",
    "IFRAME",
    "IMG",
    "INPUT",
    "INS",
    "KBD",
    "LABEL",
    "LEGEND",
    "LI",
    "LINK",
    "MAIN",
    "MAP",
    "MARK",
    "MENU",
    "META",
    "METER",
    "NAV",
    "NOFRAMES",
    "NOSCRIPT",
    "OBJECT",
    "OL",
    "OPTGROUP",
    "OPTION",
    "OUTPUT",
    "P",
    "PARAM",
    "PICTURE",
    "PRE",
    "PROGRESS",
    "Q",
    "RP",
    "RT",
    "RUBY",
    "S",
    "SAMP",
    "SCRIPT",
    "SEARCH",
    "SECTION",
    "SELECT",
    "SMALL",
    "SOURCE",
    "SPAN",
    "STRIKE",
    "STRONG",
    "STYLE",
    "SUB",
    "SUMMARY",
    "SUP",
    "SVG",
    "TABLE",
    "TBODY",
    "TD",
    "TEMPLATE",
    "TEXTAREA",
    "TFOOT",
    "TH",
    "THEAD",
    "TIME",
    "TITLE",
    "TR",
    "TRACK",
    "TT",
    "U",
    "UL",
    "VAR",
    "VIDEO",
    "WBR",
];

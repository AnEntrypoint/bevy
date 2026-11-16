use bevy::color::Srgba;
use bevy::math::Rot2;
use bevy::prelude::*;
use std::collections::BTreeMap;

const VOID: &[&str] = &["area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "wbr"];
const RAW: &[&str] = &["style", "script"];
const SKIPPED: &[&str] = &["script", "style", "template", "img", "head", "title", "noscript", "link", "meta", "input", "canvas", "video", "audio"];
const INLINE: &[&str] = &["span", "b", "i", "em", "strong", "u", "small", "a", "label", "code", "font", "sub", "sup", "mark"];
const BREAK: char = '\u{0}';

#[derive(Component)]
pub struct HtmlUiRoot;

#[derive(Resource, Default)]
pub struct HtmlUiSource {
    html: String,
    applied: Option<String>,
}

impl HtmlUiSource {
    pub fn update(&mut self, html: &str) {
        if self.html != html {
            self.html = html.to_string();
        }
    }
}

#[derive(Resource, Default, Clone, Debug)]
pub struct HtmlUiStatus {
    pub nodes: usize,
    pub warnings: Vec<String>,
}

pub struct HtmlUiPlugin;

impl Plugin for HtmlUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HtmlUiSource>()
            .init_resource::<HtmlUiStatus>()
            .add_systems(Update, sync_html_ui);
    }
}

fn sync_html_ui(
    mut commands: Commands,
    mut source: ResMut<HtmlUiSource>,
    mut status: ResMut<HtmlUiStatus>,
    roots: Query<Entity, With<HtmlUiRoot>>,
) {
    if source.applied.as_deref() == Some(source.html.as_str()) {
        return;
    }
    for root in &roots {
        commands.entity(root).despawn();
    }
    source.applied = Some(source.html.clone());
    *status = HtmlUiStatus::default();
    if source.html.trim().is_empty() {
        return;
    }
    let built = build(&source.html);
    status.nodes = built.spec.count();
    status.warnings = built.warnings;
    let root = spawn(&mut commands, None, &built.spec);
    commands.entity(root).insert(HtmlUiRoot);
}

#[derive(Clone, Debug)]
pub enum UiSpec {
    Box { style: BoxStyle, children: Vec<UiSpec> },
    Text { text: String, color: Option<Color>, size: f32 },
    Shape { style: ShapeStyle },
}

impl UiSpec {
    pub fn count(&self) -> usize {
        match self {
            UiSpec::Box { children, .. } => 1 + children.iter().map(UiSpec::count).sum::<usize>(),
            UiSpec::Text { .. } | UiSpec::Shape { .. } => 1,
        }
    }
}

#[derive(Clone, Debug)]
pub struct BoxStyle {
    row: bool,
    align: AlignItems,
    justify: JustifyContent,
    column_gap: f32,
    row_gap: f32,
    padding: [f32; 4],
    width: Val,
    height: Val,
    inset: Option<[Val; 4]>,
    background: Option<Color>,
}

impl BoxStyle {
    fn root() -> Self {
        Self {
            row: false,
            align: AlignItems::Stretch,
            justify: JustifyContent::FlexStart,
            column_gap: 0.0,
            row_gap: 0.0,
            padding: [0.0; 4],
            width: Val::Auto,
            height: Val::Auto,
            inset: Some([Val::Px(0.0), Val::Px(0.0), Val::Px(0.0), Val::Px(0.0)]),
            background: None,
        }
    }

    fn node(&self) -> Node {
        let inset = self.inset.clone().unwrap_or([Val::Auto, Val::Auto, Val::Auto, Val::Auto]);
        Node {
            flex_direction: if self.row { FlexDirection::Row } else { FlexDirection::Column },
            align_items: self.align,
            justify_content: self.justify,
            column_gap: Val::Px(self.column_gap),
            row_gap: Val::Px(self.row_gap),
            padding: UiRect::new(
                Val::Px(self.padding[3]),
                Val::Px(self.padding[1]),
                Val::Px(self.padding[0]),
                Val::Px(self.padding[2]),
            ),
            width: self.width.clone(),
            height: self.height.clone(),
            position_type: if self.inset.is_some() { PositionType::Absolute } else { PositionType::Relative },
            top: inset[0].clone(),
            right: inset[1].clone(),
            bottom: inset[2].clone(),
            left: inset[3].clone(),
            ..default()
        }
    }
}

#[derive(Clone, Debug)]
pub struct ShapeStyle {
    left: Val,
    top: Val,
    width: Val,
    height: Val,
    rotation: f32,
    round: bool,
    fill: Option<Color>,
    stroke: Option<Color>,
    stroke_width: f32,
}

impl ShapeStyle {
    fn node(&self) -> Node {
        Node {
            position_type: PositionType::Absolute,
            left: self.left.clone(),
            top: self.top.clone(),
            width: self.width.clone(),
            height: self.height.clone(),
            border: UiRect::all(Val::Px(if self.stroke.is_some() { self.stroke_width } else { 0.0 })),
            border_radius: if self.round { BorderRadius::MAX } else { BorderRadius::default() },
            ..default()
        }
    }
}

pub fn spawn(commands: &mut Commands, parent: Option<Entity>, spec: &UiSpec) -> Entity {
    let id = match spec {
        UiSpec::Box { style, children } => {
            let id = {
                let mut entity = commands.spawn(style.node());
                if let Some(background) = style.background {
                    entity.insert(BackgroundColor(background));
                }
                entity.id()
            };
            for child in children {
                spawn(commands, Some(id), child);
            }
            id
        }
        UiSpec::Shape { style } => {
            let mut entity = commands.spawn(style.node());
            if let Some(fill) = style.fill {
                entity.insert(BackgroundColor(fill));
            }
            if let Some(stroke) = style.stroke {
                entity.insert(BorderColor::all(stroke));
            }
            if style.rotation != 0.0 {
                entity.insert(UiTransform::from_rotation(Rot2::radians(style.rotation)));
            }
            entity.id()
        }
        UiSpec::Text { text, color, size } => commands
            .spawn((
                Node::default(),
                Text::new(text.clone()),
                TextFont { font_size: FontSize::Px(*size), ..default() },
                TextColor(color.unwrap_or(Color::WHITE)),
            ))
            .id(),
    };
    if let Some(parent) = parent {
        commands.entity(parent).add_child(id);
    }
    id
}

pub struct Built {
    pub spec: UiSpec,
    pub warnings: Vec<String>,
}

pub fn build(html: &str) -> Built {
    let (dom, css) = parse(html);
    let rules = parse_css(&css);
    let inherit = Inherit { vars: BTreeMap::new(), color: Some(Color::WHITE), font_size: 16.0 };
    let mut dropped = BTreeMap::new();
    let children = convert(&dom, &mut Vec::new(), &inherit, &rules, &mut dropped);
    let warnings = dropped
        .into_iter()
        .map(|(tag, count)| format!("unsupported element <{tag}> dropped ({count})"))
        .collect();
    Built { spec: UiSpec::Box { style: BoxStyle::root(), children }, warnings }
}

#[derive(Clone, Debug)]
enum Dom {
    Element { tag: String, attrs: Vec<(String, String)>, children: Vec<Dom> },
    Text(String),
}

struct Frame {
    tag: String,
    attrs: Vec<(String, String)>,
    children: Vec<Dom>,
}

fn parse(html: &str) -> (Vec<Dom>, String) {
    let mut styles = String::new();
    let mut stack = vec![Frame { tag: String::new(), attrs: Vec::new(), children: Vec::new() }];
    let mut text = String::new();
    let mut rest = html;
    while !rest.is_empty() {
        if let Some(after) = rest.strip_prefix("<!--") {
            rest = after.find("-->").map(|end| &after[end + 3..]).unwrap_or("");
            continue;
        }
        if rest.starts_with("<!") || rest.starts_with("<?") {
            rest = rest.find('>').map(|end| &rest[end + 1..]).unwrap_or("");
            continue;
        }
        if rest.starts_with("</") && rest[2..].starts_with(|c: char| c.is_ascii_alphabetic()) {
            flush(&mut text, &mut stack);
            let end = rest.find('>').unwrap_or(rest.len());
            let name = rest[2..end].trim().to_ascii_lowercase();
            rest = &rest[(end + 1).min(rest.len())..];
            if let Some(position) = stack.iter().rposition(|frame| frame.tag == name) {
                while stack.len() > position {
                    close(&mut stack);
                }
            }
            continue;
        }
        if rest.starts_with('<') && rest[1..].starts_with(|c: char| c.is_ascii_alphabetic()) {
            flush(&mut text, &mut stack);
            let (tag, attrs, self_closing, after) = read_tag(&rest[1..]);
            rest = after;
            if RAW.contains(&tag.as_str()) && !self_closing {
                let end = rest.to_ascii_lowercase().find(&format!("</{tag}")).unwrap_or(rest.len());
                if tag == "style" {
                    styles.push_str(&rest[..end]);
                    styles.push('\n');
                }
                rest = rest[end..].find('>').map(|index| &rest[end + index + 1..]).unwrap_or("");
                continue;
            }
            if self_closing || VOID.contains(&tag.as_str()) {
                if let Some(frame) = stack.last_mut() {
                    frame.children.push(Dom::Element { tag, attrs, children: Vec::new() });
                }
            } else {
                stack.push(Frame { tag, attrs, children: Vec::new() });
            }
            continue;
        }
        let Some(ch) = rest.chars().next() else { break };
        text.push(ch);
        rest = &rest[ch.len_utf8()..];
    }
    flush(&mut text, &mut stack);
    while stack.len() > 1 {
        close(&mut stack);
    }
    let root = stack.pop().map(|frame| frame.children).unwrap_or_default();
    (root, styles)
}

fn flush(text: &mut String, stack: &mut [Frame]) {
    if text.is_empty() {
        return;
    }
    if let Some(frame) = stack.last_mut() {
        frame.children.push(Dom::Text(decode(text)));
    }
    text.clear();
}

fn close(stack: &mut Vec<Frame>) {
    let Some(frame) = stack.pop() else { return };
    if let Some(parent) = stack.last_mut() {
        parent.children.push(Dom::Element { tag: frame.tag, attrs: frame.attrs, children: frame.children });
    }
}

fn read_tag(input: &str) -> (String, Vec<(String, String)>, bool, &str) {
    let name_end = input.find(|c: char| c.is_whitespace() || c == '>' || c == '/').unwrap_or(input.len());
    let tag = input[..name_end].to_ascii_lowercase();
    let mut rest = &input[name_end..];
    let mut attrs = Vec::new();
    let mut self_closing = false;
    loop {
        rest = rest.trim_start();
        if let Some(after) = rest.strip_prefix("/>") {
            self_closing = true;
            rest = after;
            break;
        }
        if let Some(after) = rest.strip_prefix('>') {
            rest = after;
            break;
        }
        if rest.is_empty() {
            break;
        }
        if let Some(after) = rest.strip_prefix('/') {
            rest = after;
            continue;
        }
        let name_len = rest.find(|c: char| c.is_whitespace() || c == '=' || c == '>' || c == '/').unwrap_or(rest.len()).max(1);
        let name = rest[..name_len].to_ascii_lowercase();
        rest = &rest[name_len..];
        let trimmed = rest.trim_start();
        let value = match trimmed.strip_prefix('=') {
            Some(after_eq) => {
                let after_eq = after_eq.trim_start();
                let quote = after_eq.chars().next().filter(|c| *c == '"' || *c == '\'');
                match quote {
                    Some(quote) => {
                        let body = &after_eq[1..];
                        let end = body.find(quote).unwrap_or(body.len());
                        rest = &body[(end + 1).min(body.len())..];
                        decode(&body[..end])
                    }
                    None => {
                        let end = after_eq.find(|c: char| c.is_whitespace() || c == '>').unwrap_or(after_eq.len());
                        rest = &after_eq[end..];
                        decode(&after_eq[..end])
                    }
                }
            }
            None => String::new(),
        };
        attrs.push((name, value));
    }
    (tag, attrs, self_closing, rest)
}

fn decode(text: &str) -> String {
    text.replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

fn attr<'a>(attrs: &'a [(String, String)], name: &str) -> Option<&'a str> {
    attrs.iter().find(|(key, _)| key == name).map(|(_, value)| value.as_str())
}

struct Compound {
    tag: Option<String>,
    id: Option<String>,
    classes: Vec<String>,
}

impl Compound {
    fn matches(&self, elem: &Elem) -> bool {
        self.tag.as_ref().is_none_or(|tag| *tag == elem.tag)
            && self.id.as_ref().is_none_or(|id| elem.id.as_deref() == Some(id.as_str()))
            && self.classes.iter().all(|class| elem.classes.contains(class))
    }
}

struct Elem {
    tag: String,
    id: Option<String>,
    classes: Vec<String>,
}

impl Elem {
    fn new(tag: &str, attrs: &[(String, String)]) -> Self {
        Self {
            tag: tag.to_string(),
            id: attr(attrs, "id").map(str::to_string),
            classes: attr(attrs, "class").unwrap_or("").split_whitespace().map(str::to_string).collect(),
        }
    }
}

struct Rule {
    parts: Vec<(char, Compound)>,
    spec: (usize, usize, usize),
    order: usize,
    decls: Vec<(String, String)>,
}

fn parse_compound(token: &str) -> Option<Compound> {
    if token.chars().any(|c| ":[]*+~()".contains(c)) {
        return None;
    }
    let mut compound = Compound { tag: None, id: None, classes: Vec::new() };
    let mut kind = 't';
    let mut name = String::new();
    for c in token.chars().chain(std::iter::once('.')) {
        if c == '.' || c == '#' {
            if !name.is_empty() {
                match kind {
                    't' => compound.tag = Some(name.to_ascii_lowercase()),
                    '#' => compound.id = Some(name.clone()),
                    _ => compound.classes.push(name.clone()),
                }
            }
            name.clear();
            kind = if c == '#' { '#' } else { '.' };
        } else {
            name.push(c);
        }
    }
    Some(compound)
}

fn parse_selector(text: &str) -> Option<(Vec<(char, Compound)>, (usize, usize, usize))> {
    let spaced = text.replace('>', " > ");
    let mut parts = Vec::new();
    let mut combinator = ' ';
    let mut spec = (0, 0, 0);
    for token in spaced.split_whitespace() {
        if token == ">" {
            combinator = '>';
            continue;
        }
        let compound = parse_compound(token)?;
        spec.0 += usize::from(compound.id.is_some());
        spec.1 += compound.classes.len();
        spec.2 += usize::from(compound.tag.is_some());
        parts.push((combinator, compound));
        combinator = ' ';
    }
    (!parts.is_empty()).then_some((parts, spec))
}

fn matches_at(parts: &[(char, Compound)], index: usize, path: &[Elem], position: usize) -> bool {
    if !parts[index].1.matches(&path[position]) {
        return false;
    }
    if index == 0 {
        return true;
    }
    match parts[index].0 {
        '>' => position > 0 && matches_at(parts, index - 1, path, position - 1),
        _ => (0..position).rev().any(|ancestor| matches_at(parts, index - 1, path, ancestor)),
    }
}

fn strip_comments(css: &str) -> String {
    let mut out = String::new();
    let mut rest = css;
    while let Some(start) = rest.find("/*") {
        out.push_str(&rest[..start]);
        rest = rest[start..].find("*/").map(|end| &rest[start + end + 2..]).unwrap_or("");
    }
    out.push_str(rest);
    out
}

fn block_end(text: &str) -> Option<usize> {
    let mut depth = 0;
    for (index, c) in text.char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

fn parse_css(css: &str) -> Vec<Rule> {
    let css = strip_comments(css);
    let mut rules: Vec<Rule> = Vec::new();
    let mut rest = css.as_str();
    while let Some(open) = rest.find('{') {
        let prelude = rest[..open].trim().to_string();
        let Some(close) = block_end(&rest[open..]) else { break };
        let body = &rest[open + 1..open + close];
        rest = &rest[open + close + 1..];
        if prelude.starts_with('@') {
            continue;
        }
        let decls = parse_decls(body);
        for selector in prelude.split(',') {
            if let Some((parts, spec)) = parse_selector(selector.trim()) {
                let order = rules.len();
                rules.push(Rule { parts, spec, order, decls: decls.clone() });
            }
        }
    }
    rules
}

fn parse_decls(body: &str) -> Vec<(String, String)> {
    let mut pieces = Vec::new();
    let mut depth = 0i32;
    let mut current = String::new();
    for c in body.chars() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            ';' if depth == 0 => {
                pieces.push(std::mem::take(&mut current));
                continue;
            }
            _ => {}
        }
        current.push(c);
    }
    pieces.push(current);
    pieces
        .iter()
        .filter_map(|piece| {
            let (name, value) = piece.split_once(':')?;
            let name = name.trim();
            let name = if name.starts_with("--") { name.to_string() } else { name.to_ascii_lowercase() };
            let value = value.trim().trim_end_matches("!important").trim().to_string();
            (!name.is_empty() && !value.is_empty()).then_some((name, value))
        })
        .collect()
}

fn cascade(path: &[Elem], rules: &[Rule], inline: &str) -> BTreeMap<String, String> {
    let mut matched: Vec<&Rule> = rules
        .iter()
        .filter(|rule| matches_at(&rule.parts, rule.parts.len() - 1, path, path.len() - 1))
        .collect();
    matched.sort_by_key(|rule| (rule.spec, rule.order));
    let mut props = BTreeMap::new();
    for rule in matched {
        props.extend(rule.decls.iter().cloned());
    }
    props.extend(parse_decls(inline));
    props
}

fn substitute(value: &str, vars: &BTreeMap<String, String>) -> String {
    let mut out = value.to_string();
    for _ in 0..8 {
        let Some(start) = out.find("var(") else { break };
        let Some(close) = out[start..].find(')').map(|end| start + end) else { break };
        let inner = out[start + 4..close].to_string();
        let (name, fallback) = match inner.split_once(',') {
            Some((name, fallback)) => (name.trim(), fallback.trim()),
            None => (inner.trim(), ""),
        };
        let replacement = vars.get(name).cloned().unwrap_or_else(|| fallback.to_string());
        out.replace_range(start..=close, &replacement);
    }
    out
}

#[derive(Clone, Default)]
struct Inherit {
    vars: BTreeMap<String, String>,
    color: Option<Color>,
    font_size: f32,
}

struct Scope {
    props: BTreeMap<String, String>,
    inherit: Inherit,
    opacity: f32,
}

impl Scope {
    fn new(props: BTreeMap<String, String>, parent: &Inherit) -> Self {
        let mut vars = parent.vars.clone();
        for (name, value) in &props {
            if name.starts_with("--") {
                vars.insert(name.clone(), value.clone());
            }
        }
        let mut scope = Self {
            props,
            inherit: Inherit { vars, color: parent.color, font_size: parent.font_size },
            opacity: 1.0,
        };
        scope.opacity = scope.get("opacity").and_then(|value| value.parse::<f32>().ok()).unwrap_or(1.0).clamp(0.0, 1.0);
        scope.inherit.color = scope.get("color").and_then(|value| parse_color(&value)).map(|color| with_opacity(color, scope.opacity)).or(parent.color);
        scope.inherit.font_size = scope.get("font-size").and_then(|value| parse_px(&value)).unwrap_or(parent.font_size);
        scope
    }

    fn get(&self, name: &str) -> Option<String> {
        self.props.get(name).map(|value| substitute(value, &self.inherit.vars))
    }
}

fn with_opacity(color: Color, opacity: f32) -> Color {
    let srgba: Srgba = color.to_srgba();
    Color::srgba(srgba.red, srgba.green, srgba.blue, srgba.alpha * opacity)
}

fn parse_hex(hex: &str) -> Option<Color> {
    let digits: Vec<u8> = match hex.len() {
        3 | 4 => hex.chars().map(|c| c.to_digit(16).map(|d| (d * 17) as u8)).collect::<Option<Vec<u8>>>()?,
        6 | 8 => (0..hex.len())
            .step_by(2)
            .map(|index| u8::from_str_radix(&hex[index..index + 2], 16).ok())
            .collect::<Option<Vec<u8>>>()?,
        _ => return None,
    };
    let alpha = digits.get(3).copied().unwrap_or(255) as f32 / 255.0;
    Some(Color::srgba(digits[0] as f32 / 255.0, digits[1] as f32 / 255.0, digits[2] as f32 / 255.0, alpha))
}

fn parse_color(value: &str) -> Option<Color> {
    let value = value.trim().to_ascii_lowercase();
    if let Some(hex) = value.strip_prefix('#') {
        return parse_hex(hex);
    }
    if let Some(args) = value.strip_prefix("rgba(").or_else(|| value.strip_prefix("rgb(")).and_then(|args| args.strip_suffix(')')) {
        let parts: Vec<&str> = args.split(|c: char| c == ',' || c == '/' || c.is_whitespace()).filter(|part| !part.is_empty()).collect();
        if parts.len() < 3 {
            return None;
        }
        let channel = |part: &str| -> Option<f32> {
            match part.strip_suffix('%') {
                Some(percent) => percent.parse::<f32>().ok().map(|value| value / 100.0),
                None => part.parse::<f32>().ok().map(|value| value / 255.0),
            }
        };
        let alpha = match parts.get(3) {
            Some(part) => match part.strip_suffix('%') {
                Some(percent) => percent.parse::<f32>().ok()? / 100.0,
                None => part.parse::<f32>().ok()?,
            },
            None => 1.0,
        };
        return Some(Color::srgba(
            channel(parts[0])?.clamp(0.0, 1.0),
            channel(parts[1])?.clamp(0.0, 1.0),
            channel(parts[2])?.clamp(0.0, 1.0),
            alpha.clamp(0.0, 1.0),
        ));
    }
    match value.as_str() {
        "transparent" => Some(Color::srgba(0.0, 0.0, 0.0, 0.0)),
        "white" => Some(Color::WHITE),
        "black" => Some(Color::BLACK),
        "red" => Some(Color::srgb(1.0, 0.0, 0.0)),
        "green" => Some(Color::srgb(0.0, 0.5, 0.0)),
        "blue" => Some(Color::srgb(0.0, 0.0, 1.0)),
        "gray" | "grey" => Some(Color::srgb(0.5, 0.5, 0.5)),
        _ => None,
    }
}

fn parse_px(value: &str) -> Option<f32> {
    let number = value.trim().strip_suffix("px").unwrap_or(value.trim());
    number.parse::<f32>().ok()
}

fn parse_val(value: &str) -> Option<Val> {
    let value = value.trim();
    if value == "auto" {
        return Some(Val::Auto);
    }
    if let Some(percent) = value.strip_suffix('%') {
        return percent.parse::<f32>().ok().map(Val::Percent);
    }
    parse_px(value).map(Val::Px)
}

fn shorthand<T: Clone>(value: &str, parse: impl Fn(&str) -> Option<T>) -> Option<[T; 4]> {
    let parts = value.split_whitespace().map(&parse).collect::<Option<Vec<T>>>()?;
    Some(match parts.len() {
        1 => {
            let a = parts[0].clone();
            [a.clone(), a.clone(), a.clone(), a]
        }
        2 => [parts[0].clone(), parts[1].clone(), parts[0].clone(), parts[1].clone()],
        3 => [parts[0].clone(), parts[1].clone(), parts[2].clone(), parts[1].clone()],
        4 => [parts[0].clone(), parts[1].clone(), parts[2].clone(), parts[3].clone()],
        _ => return None,
    })
}

fn parse_align(value: &str) -> Option<AlignItems> {
    match value {
        "flex-start" | "start" | "self-start" => Some(AlignItems::FlexStart),
        "flex-end" | "end" | "self-end" => Some(AlignItems::FlexEnd),
        "center" => Some(AlignItems::Center),
        "stretch" | "normal" => Some(AlignItems::Stretch),
        "baseline" => Some(AlignItems::Baseline),
        _ => None,
    }
}

fn parse_justify(value: &str) -> Option<JustifyContent> {
    match value {
        "flex-start" | "start" => Some(JustifyContent::FlexStart),
        "flex-end" | "end" => Some(JustifyContent::FlexEnd),
        "center" => Some(JustifyContent::Center),
        "space-between" => Some(JustifyContent::SpaceBetween),
        "space-around" => Some(JustifyContent::SpaceAround),
        "space-evenly" => Some(JustifyContent::SpaceEvenly),
        _ => None,
    }
}

fn box_style(scope: &Scope) -> BoxStyle {
    let display = scope.get("display").unwrap_or_default();
    let flex = display == "flex" || display == "inline-flex";
    let direction = scope.get("flex-direction").unwrap_or_default();
    let row = flex && !direction.starts_with("column");
    let align = scope.get("align-items").and_then(|value| parse_align(&value)).unwrap_or(AlignItems::Stretch);
    let justify = scope.get("justify-content").and_then(|value| parse_justify(&value)).unwrap_or(JustifyContent::FlexStart);
    let (gap_row, gap_column) = match scope.get("gap").map(|value| value.split_whitespace().filter_map(parse_px).collect::<Vec<f32>>()) {
        Some(parts) if parts.len() == 2 => (parts[0], parts[1]),
        Some(parts) if !parts.is_empty() => (parts[0], parts[0]),
        _ => (0.0, 0.0),
    };
    let row_gap = scope.get("row-gap").and_then(|value| parse_px(&value)).unwrap_or(gap_row);
    let column_gap = scope.get("column-gap").and_then(|value| parse_px(&value)).unwrap_or(gap_column);
    let mut padding = scope.get("padding").and_then(|value| shorthand(&value, parse_px)).unwrap_or([0.0; 4]);
    for (index, side) in ["padding-top", "padding-right", "padding-bottom", "padding-left"].iter().enumerate() {
        if let Some(value) = scope.get(side).and_then(|value| parse_px(&value)) {
            padding[index] = value;
        }
    }
    let width = scope.get("width").and_then(|value| parse_val(&value)).unwrap_or(Val::Auto);
    let height = scope.get("height").and_then(|value| parse_val(&value)).unwrap_or(Val::Auto);
    let position = scope.get("position").unwrap_or_default();
    let inset = if position == "absolute" || position == "fixed" {
        let mut sides = scope.get("inset").and_then(|value| shorthand(&value, parse_val)).unwrap_or([Val::Auto, Val::Auto, Val::Auto, Val::Auto]);
        for (index, side) in ["top", "right", "bottom", "left"].iter().enumerate() {
            if let Some(value) = scope.get(side).and_then(|value| parse_val(&value)) {
                sides[index] = value;
            }
        }
        Some(sides)
    } else {
        None
    };
    let background = scope
        .get("background-color")
        .or_else(|| scope.get("background"))
        .filter(|value| !value.contains("gradient"))
        .and_then(|value| parse_color(&value).or_else(|| value.split_whitespace().find_map(parse_color)))
        .map(|color| with_opacity(color, scope.opacity));
    BoxStyle {
        row,
        align,
        justify,
        column_gap,
        row_gap,
        padding,
        width,
        height,
        inset,
        background,
    }
}

fn flush_run(run: &mut String, out: &mut Vec<UiSpec>, inherit: &Inherit) {
    let mut collapsed = String::new();
    let mut spaced = false;
    for c in run.chars() {
        if c.is_whitespace() {
            if !spaced {
                collapsed.push(' ');
            }
            spaced = true;
        } else {
            collapsed.push(c);
            spaced = false;
        }
    }
    run.clear();
    let text = collapsed.split(BREAK).map(str::trim).collect::<Vec<&str>>().join("\n");
    if text.trim().is_empty() {
        return;
    }
    out.push(UiSpec::Text { text, color: inherit.color, size: inherit.font_size });
}

#[derive(Clone, Copy)]
struct Canvas {
    size: Option<(f32, f32)>,
}

impl Canvas {
    fn x(&self, value: f32) -> Val {
        match self.size {
            Some((width, _)) => Val::Percent(value / width * 100.0),
            None => Val::Px(value),
        }
    }

    fn y(&self, value: f32) -> Val {
        match self.size {
            Some((_, height)) => Val::Percent(value / height * 100.0),
            None => Val::Px(value),
        }
    }
}

fn canvas(attrs: &[(String, String)]) -> Canvas {
    let view: Vec<f32> = attr(attrs, "viewBox").unwrap_or("").split(|c: char| c == ' ' || c == ',').filter_map(|part| part.parse::<f32>().ok()).collect();
    match view.as_slice() {
        [_, _, width, height] if *width > 0.0 && *height > 0.0 => Canvas { size: Some((*width, *height)) },
        _ => Canvas { size: None },
    }
}

fn fit_svg(style: &mut BoxStyle, attrs: &[(String, String)]) {
    let length = |name: &str| attr(attrs, name).and_then(|value| value.trim_end_matches("px").parse::<f32>().ok()).map(Val::Px);
    if matches!(style.width, Val::Auto) {
        style.width = length("width").unwrap_or(Val::Auto);
    }
    if matches!(style.height, Val::Auto) {
        style.height = length("height").unwrap_or(Val::Auto);
    }
}

fn number(attrs: &[(String, String)], name: &str, default: f32) -> f32 {
    attr(attrs, name).and_then(|value| value.trim_end_matches("px").parse::<f32>().ok()).unwrap_or(default)
}

fn paint(attrs: &[(String, String)], name: &str, default: Option<&str>) -> Option<Color> {
    let color = parse_color(attr(attrs, name).or(default)?)?;
    let opacity = attr(attrs, &format!("{name}-opacity")).and_then(|value| value.parse::<f32>().ok()).unwrap_or(1.0);
    Some(with_opacity(color, opacity))
}

fn svg_shapes(nodes: &[Dom], canvas: Canvas, dropped: &mut BTreeMap<String, usize>) -> Vec<UiSpec> {
    let mut out = Vec::new();
    for node in nodes {
        let Dom::Element { tag, attrs, children } = node else { continue };
        let stroke_width = number(attrs, "stroke-width", 1.0);
        let stroke = paint(attrs, "stroke", None);
        let fill = paint(attrs, "fill", Some("black"));
        let shape = match tag.as_str() {
            "circle" => {
                let (cx, cy, radius) = (number(attrs, "cx", 0.0), number(attrs, "cy", 0.0), number(attrs, "r", 0.0));
                ShapeStyle {
                    left: canvas.x(cx - radius),
                    top: canvas.y(cy - radius),
                    width: canvas.x(2.0 * radius),
                    height: canvas.y(2.0 * radius),
                    rotation: 0.0,
                    round: true,
                    fill,
                    stroke,
                    stroke_width,
                }
            }
            "rect" => ShapeStyle {
                left: canvas.x(number(attrs, "x", 0.0)),
                top: canvas.y(number(attrs, "y", 0.0)),
                width: canvas.x(number(attrs, "width", 0.0)),
                height: canvas.y(number(attrs, "height", 0.0)),
                rotation: 0.0,
                round: false,
                fill,
                stroke,
                stroke_width,
            },
            "line" => {
                let (x1, y1) = (number(attrs, "x1", 0.0), number(attrs, "y1", 0.0));
                let (dx, dy) = (number(attrs, "x2", 0.0) - x1, number(attrs, "y2", 0.0) - y1);
                let length = (dx * dx + dy * dy).sqrt();
                let (mid_x, mid_y) = (x1 + dx / 2.0, y1 + dy / 2.0);
                ShapeStyle {
                    left: canvas.x(mid_x - length / 2.0),
                    top: canvas.y(mid_y - stroke_width / 2.0),
                    width: canvas.x(length),
                    height: canvas.y(stroke_width),
                    rotation: dy.atan2(dx),
                    round: false,
                    fill: stroke,
                    stroke: None,
                    stroke_width: 0.0,
                }
            }
            "g" => {
                out.extend(svg_shapes(children, canvas, dropped));
                continue;
            }
            other => {
                *dropped.entry(other.to_string()).or_insert(0) += 1;
                continue;
            }
        };
        out.push(UiSpec::Shape { style: shape });
    }
    out
}

fn convert(nodes: &[Dom], path: &mut Vec<Elem>, inherit: &Inherit, rules: &[Rule], dropped: &mut BTreeMap<String, usize>) -> Vec<UiSpec> {
    let mut out = Vec::new();
    let mut run = String::new();
    for node in nodes {
        match node {
            Dom::Text(text) => run.push_str(text),
            Dom::Element { tag, attrs, children } => {
                if SKIPPED.contains(&tag.as_str()) {
                    *dropped.entry(tag.clone()).or_insert(0) += 1;
                    continue;
                }
                if tag == "br" {
                    run.push(BREAK);
                    continue;
                }
                path.push(Elem::new(tag, attrs));
                let props = cascade(path, rules, attr(attrs, "style").unwrap_or(""));
                let scope = Scope::new(props, inherit);
                if scope.get("display").as_deref() == Some("none") {
                    path.pop();
                    continue;
                }
                if scope.props.is_empty() && INLINE.contains(&tag.as_str()) {
                    for spec in convert(children, path, &scope.inherit, rules, dropped) {
                        match spec {
                            UiSpec::Text { text, .. } => run.push_str(&text),
                            other => {
                                flush_run(&mut run, &mut out, inherit);
                                out.push(other);
                            }
                        }
                    }
                } else {
                    flush_run(&mut run, &mut out, inherit);
                    let mut style = box_style(&scope);
                    let inner = if tag == "svg" {
                        fit_svg(&mut style, attrs);
                        svg_shapes(children, canvas(attrs), dropped)
                    } else {
                        convert(children, path, &scope.inherit, rules, dropped)
                    };
                    out.push(UiSpec::Box { style, children: inner });
                }
                path.pop();
            }
        }
    }
    flush_run(&mut run, &mut out, inherit);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const NAMETAG: &str = include_str!("../testdata/ui-html/nametag-sample.html");

    #[test]
    fn nametag_sample_maps_to_expected_node_count() {
        let spec = build(NAMETAG).spec;
        assert_eq!(spec.count(), 13);
    }

    #[test]
    fn nametag_sample_has_no_warnings() {
        assert!(build(NAMETAG).warnings.is_empty());
    }

    #[test]
    fn spawned_nodes_match_spec_count() {
        let spec = build(NAMETAG).spec;
        let expected = spec.count();
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        let spec_for_system = spec.clone();
        app.add_systems(Startup, move |mut commands: Commands| {
            spawn(&mut commands, None, &spec_for_system);
        });
        app.update();
        let nodes = app.world_mut().query::<&Node>().iter(app.world()).count();
        assert_eq!(nodes, expected);
    }

    #[test]
    fn nametag_text_and_styles_are_mapped() {
        let spec = build(NAMETAG).spec;
        let mut texts = Vec::new();
        collect_texts(&spec, &mut texts);
        assert!(texts.iter().any(|text| text == "Mira"));
        assert!(texts.iter().any(|text| text == "hi"));
        let UiSpec::Box { children, .. } = &spec else { panic!("root is a box") };
        let UiSpec::Box { style, .. } = &children[0] else { panic!("tag is a box") };
        assert!(!style.row);
        assert!(matches!(style.inset, None));
    }

    #[test]
    fn display_none_and_unsupported_elements_are_dropped() {
        let spec = build("<style>.gone{display:none}</style><div class=gone>x</div><svg><rect/></svg><div>y</div>").spec;
        assert_eq!(spec.count(), 5);
    }

    #[test]
    fn svg_shapes_map_to_absolute_nodes() {
        let built = build(r##"<svg width="10" height="10" viewBox="0 0 10 10"><circle cx="5" cy="5" r="4" fill="none" stroke="#ffffff"/><line x1="0" y1="0" x2="10" y2="0" stroke="#ff0000"/><rect x="1" y="1" width="2" height="2" fill="#00ff00"/></svg>"##);
        assert!(built.warnings.is_empty());
        assert_eq!(built.spec.count(), 5);
    }

    #[test]
    fn svg_unsupported_children_are_reported() {
        let built = build(r#"<svg><path d="M0 0"/><g><circle r="1"/></g></svg>"#);
        assert_eq!(built.warnings, vec!["unsupported element <path> dropped (1)".to_string()]);
        assert_eq!(built.spec.count(), 3);
    }

    #[test]
    fn unsupported_elements_are_reported_not_silent() {
        let built = build("<svg><rect/></svg><svg></svg><img src=x><div>y</div>");
        assert_eq!(built.warnings, vec!["unsupported element <img> dropped (1)".to_string()]);
    }

    #[test]
    fn flex_and_inline_style_are_mapped() {
        let built = build(r#"<div id="bar" style="display:flex;gap:4px;padding:2px 6px;width:50%"><span>a</span><span>b</span></div>"#);
        let UiSpec::Box { children, .. } = &built.spec else { panic!("root is a box") };
        let UiSpec::Box { style, children } = &children[0] else { panic!("bar is a box") };
        assert!(style.row);
        assert_eq!(style.column_gap, 4.0);
        assert_eq!(style.padding, [2.0, 6.0, 2.0, 6.0]);
        assert_eq!(style.width, Val::Percent(50.0));
        assert_eq!(children.len(), 1);
    }

    #[test]
    fn source_update_only_changes_on_new_html() {
        let mut source = HtmlUiSource::default();
        source.update("<div>a</div>");
        assert_eq!(source.html, "<div>a</div>");
        source.applied = Some("<div>a</div>".to_string());
        source.update("<div>a</div>");
        assert_eq!(source.applied.as_deref(), Some("<div>a</div>"));
        source.update("<div>b</div>");
        assert_eq!(source.html, "<div>b</div>");
    }

    fn collect_texts(spec: &UiSpec, out: &mut Vec<String>) {
        match spec {
            UiSpec::Box { children, .. } => children.iter().for_each(|child| collect_texts(child, out)),
            UiSpec::Text { text, .. } => out.push(text.clone()),
            UiSpec::Shape { .. } => {}
        }
    }
}

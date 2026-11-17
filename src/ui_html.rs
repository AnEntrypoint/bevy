use bevy::color::Srgba;
use crate::css_anim::{attach, parse_animation, parse_keyframes, sample, Animation, Keyframes};
use std::sync::Arc;
use bevy::math::Rot2;
use bevy::prelude::*;
use std::collections::{BTreeMap, HashMap};

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
    pub uncovered: usize,
    pub anchors: usize,
}

#[derive(Component, Clone, Copy, Debug, Default)]
pub struct HtmlTranslated;

#[derive(Component, Clone, Debug)]
pub struct HtmlAnimated {
    pub key: String,
    pub animations: Vec<Animation>,
}

#[derive(Component, Clone, Copy, Debug)]
pub struct HtmlBaseAlpha(pub f32);

#[derive(Resource, Default, Clone, Debug)]
pub struct HtmlAnimationClock {
    pub starts: HashMap<String, f32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AnimationSample {
    pub key: String,
    pub names: Vec<String>,
    pub elapsed: f32,
    pub opacity: f32,
    pub visible: bool,
}

#[derive(Resource, Default, Clone, Debug)]
pub struct HtmlAnimationStatus {
    pub samples: Vec<AnimationSample>,
}

#[derive(Component, Clone, Debug, PartialEq)]
pub struct WorldAnchor {
    pub id: String,
    pub offset: Vec3,
}

fn anchor_offset(value: Option<&str>) -> Vec3 {
    let parts = value
        .unwrap_or_default()
        .split_whitespace()
        .filter_map(|part| part.parse::<f32>().ok())
        .collect::<Vec<_>>();
    match parts.as_slice() {
        [x, y, z] => Vec3::new(*x, *y, *z),
        _ => Vec3::ZERO,
    }
}

pub struct HtmlUiPlugin;

impl Plugin for HtmlUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HtmlUiSource>()
            .init_resource::<HtmlUiStatus>()
            .init_resource::<HtmlAnimationClock>()
            .init_resource::<HtmlAnimationStatus>()
            .add_systems(Update, (sync_html_ui, animate_html).chain());
    }
}

fn sync_html_ui(
    mut commands: Commands,
    mut source: ResMut<HtmlUiSource>,
    mut status: ResMut<HtmlUiStatus>,
    roots: Query<Entity, With<HtmlUiRoot>>,
    time: Res<Time>,
    mut clock: ResMut<HtmlAnimationClock>,
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
        clock.starts.clear();
        return;
    }
    let built = build(&source.html);
    status.nodes = built.spec.count();
    status.warnings = built.warnings;
    status.uncovered = built.spec.uncovered_glyphs();
    status.anchors = built.spec.anchors();
    let keys = built.spec.animation_keys();
    clock.starts.retain(|key, _| keys.contains(key));
    let now = time.elapsed_secs();
    for key in keys {
        clock.starts.entry(key).or_insert(now);
    }
    let root = spawn(&mut commands, None, &built.spec);
    commands.entity(root).insert(HtmlUiRoot);
}

fn animate_html(
    time: Res<Time>,
    clock: Res<HtmlAnimationClock>,
    animated: Query<(Entity, &HtmlAnimated)>,
    children: Query<&Children>,
    mut paints: Query<(&HtmlBaseAlpha, Option<&mut TextColor>, Option<&mut BackgroundColor>)>,
    mut visibility: Query<&mut Visibility, Without<WorldAnchor>>,
    mut status: ResMut<HtmlAnimationStatus>,
) {
    let now = time.elapsed_secs();
    status.samples.clear();
    for (entity, animated) in &animated {
        let Some(start) = clock.starts.get(&animated.key) else {
            continue;
        };
        let elapsed = now - start;
        let sampled = sample(&animated.animations, elapsed);
        if let Ok(mut current) = visibility.get_mut(entity) {
            let wanted = if sampled.visible { Visibility::Inherited } else { Visibility::Hidden };
            if *current != wanted {
                *current = wanted;
            }
        }
        let mut stack = vec![entity];
        while let Some(node) = stack.pop() {
            if let Ok((base, text, background)) = paints.get_mut(node) {
                if let Some(mut text) = text {
                    text.0.set_alpha(base.0 * sampled.opacity);
                }
                if let Some(mut background) = background {
                    background.0.set_alpha(base.0 * sampled.opacity);
                }
            }
            if let Ok(kids) = children.get(node) {
                stack.extend(kids.iter());
            }
        }
        status.samples.push(AnimationSample {
            key: animated.key.clone(),
            names: animated.animations.iter().map(|animation| animation.name.clone()).collect(),
            elapsed,
            opacity: sampled.opacity,
            visible: sampled.visible,
        });
    }
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

    pub fn animation_keys(&self) -> Vec<String> {
        match self {
            UiSpec::Box { style, children } => style
                .animation_key
                .iter()
                .filter(|_| !style.animations.is_empty())
                .cloned()
                .chain(children.iter().flat_map(UiSpec::animation_keys))
                .collect(),
            _ => Vec::new(),
        }
    }

    fn finish_animations(&mut self, keyframes: &BTreeMap<String, Arc<Keyframes>>, path: &str) {
        if let UiSpec::Box { style, children } = self {
            if !style.animations.is_empty() {
                attach(&mut style.animations, keyframes);
                style.animations.retain(|animation| animation.keyframes.is_some());
                if style.animation_key.is_none() {
                    let names = style.animations.iter().map(|animation| animation.name.as_str()).collect::<Vec<_>>().join(",");
                    style.animation_key = Some(format!("path:{path}:{names}"));
                }
            }
            for (index, child) in children.iter_mut().enumerate() {
                child.finish_animations(keyframes, &format!("{path}/{index}"));
            }
        }
    }

    pub fn anchors(&self) -> usize {
        match self {
            UiSpec::Box { style, children } => usize::from(style.anchor.is_some()) + children.iter().map(UiSpec::anchors).sum::<usize>(),
            _ => 0,
        }
    }

    pub fn uncovered_glyphs(&self) -> usize {
        match self {
            UiSpec::Box { children, .. } => children.iter().map(UiSpec::uncovered_glyphs).sum(),
            UiSpec::Text { text, .. } => uncovered_glyph_count(text),
            UiSpec::Shape { .. } => 0,
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
    align_self: AlignSelf,
    background: Option<Color>,
    anchor: Option<WorldAnchor>,
    translate: Option<(Val, Val)>,
    animations: Vec<Animation>,
    animation_key: Option<String>,
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
            align_self: AlignSelf::Auto,
            background: None,
            anchor: None,
            translate: None,
            animations: Vec::new(),
            animation_key: None,
        }
    }

    fn node(&self) -> Node {
        let inset = self.inset.clone().unwrap_or([Val::Auto, Val::Auto, Val::Auto, Val::Auto]);
        Node {
            flex_direction: if self.row { FlexDirection::Row } else { FlexDirection::Column },
            align_items: self.align,
            align_self: self.align_self,
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
                    entity.insert((BackgroundColor(background), HtmlBaseAlpha(background.alpha())));
                }
                if !style.animations.is_empty() {
                    entity.insert(HtmlAnimated { key: style.animation_key.clone().unwrap_or_default(), animations: style.animations.clone() });
                }
                if let Some(anchor) = &style.anchor {
                    entity.insert((anchor.clone(), Visibility::Hidden));
                }
                if let Some((x, y)) = style.translate {
                    entity.insert((UiTransform::from_xy(x, y), HtmlTranslated));
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
                entity.insert((BackgroundColor(fill), HtmlBaseAlpha(fill.alpha())));
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
                HtmlBaseAlpha(color.unwrap_or(Color::WHITE).alpha()),
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
    let keyframes = parse_keyframes(&css);
    let warnings = dropped
        .into_iter()
        .map(|(tag, count)| format!("unsupported element <{tag}> dropped ({count})"))
        .collect();
    let mut spec = UiSpec::Box { style: BoxStyle::root(), children };
    spec.finish_animations(&keyframes, "");
    Built { spec, warnings }
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
    attrs.iter().find(|(key, _)| key.eq_ignore_ascii_case(name)).map(|(_, value)| value.as_str())
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

fn parse_oklch(args: &str) -> Option<Color> {
    let (main, alpha) = match args.split_once('/') {
        Some((main, alpha)) => (main, Some(alpha)),
        None => (args, None),
    };
    let values: Vec<&str> = main.split(|c: char| c.is_whitespace() || c == ',').filter(|part| !part.is_empty()).collect();
    if values.len() < 3 {
        return None;
    }
    let lightness = match values[0].strip_suffix('%') {
        Some(percent) => percent.parse::<f32>().ok()? / 100.0,
        None => values[0].parse::<f32>().ok()?,
    };
    let chroma: f32 = values[1].parse().ok()?;
    let hue = values[2].trim_end_matches("deg").parse::<f32>().ok()?.to_radians();
    let alpha = match alpha {
        Some(text) => match text.trim().strip_suffix('%') {
            Some(percent) => percent.parse::<f32>().ok()? / 100.0,
            None => text.trim().parse::<f32>().ok()?,
        },
        None => 1.0,
    };
    let (a, b) = (chroma * hue.cos(), chroma * hue.sin());
    let l = (lightness + 0.396_337_78 * a + 0.215_803_76 * b).powi(3);
    let m = (lightness - 0.105_561_346 * a - 0.063_854_17 * b).powi(3);
    let s = (lightness - 0.089_484_18 * a - 1.291_485_5 * b).powi(3);
    let linear = [
        4.076_741_7 * l - 3.307_711_6 * m + 0.230_969_93 * s,
        -1.268_438 * l + 2.609_757_4 * m - 0.341_319_4 * s,
        -0.004_196_086_3 * l - 0.703_418_6 * m + 1.707_614_7 * s,
    ];
    let encode = |channel: f32| {
        let channel = channel.clamp(0.0, 1.0);
        if channel <= 0.003_130_8 {
            12.92 * channel
        } else {
            1.055 * channel.powf(1.0 / 2.4) - 0.055
        }
    };
    Some(Color::srgba(encode(linear[0]), encode(linear[1]), encode(linear[2]), alpha.clamp(0.0, 1.0)))
}

fn parse_color(value: &str) -> Option<Color> {
    let value = value.trim().to_ascii_lowercase();
    if let Some(hex) = value.strip_prefix('#') {
        return parse_hex(hex);
    }
    if let Some(args) = value.strip_prefix("oklch(").and_then(|args| args.strip_suffix(')')) {
        return parse_oklch(args);
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
    if let Some(inner) = value.strip_prefix("calc(").and_then(|rest| rest.strip_suffix(')')) {
        return parse_calc(inner);
    }
    if let Some(percent) = value.strip_suffix('%') {
        return percent.parse::<f32>().ok().map(Val::Percent);
    }
    parse_px(value).map(Val::Px)
}

fn parse_calc(inner: &str) -> Option<Val> {
    let mut px = 0.0_f32;
    let mut percent = 0.0_f32;
    let mut sign = 1.0_f32;
    for token in inner.split_whitespace() {
        match token {
            "+" => sign = 1.0,
            "-" => sign = -1.0,
            term => {
                match term.strip_suffix('%') {
                    Some(number) => percent += sign * number.parse::<f32>().ok()?,
                    None => px += sign * parse_px(term)?,
                }
                sign = 1.0;
            }
        }
    }
    Some(if percent != 0.0 { Val::Percent(percent) } else { Val::Px(px) })
}

pub fn overlapping_pairs(rects: &[[f32; 4]]) -> Vec<(usize, usize)> {
    (0..rects.len())
        .flat_map(|first| (first + 1..rects.len()).map(move |second| (first, second)))
        .filter(|&(first, second)| {
            let (a, b) = (rects[first], rects[second]);
            a[0] < b[2] && b[0] < a[2] && a[1] < b[3] && b[1] < a[3]
        })
        .collect()
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
    let mut inset = if position == "absolute" || position == "fixed" {
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
    let shrinks = scope
        .get("width")
        .is_some_and(|value| matches!(value.trim(), "max-content" | "fit-content" | "min-content"));
    let centred = shrinks && inset.is_some() && horizontal_margins_auto(scope);
    if let Some(sides) = inset.as_mut().filter(|_| centred) {
        sides[1] = Val::Auto;
        sides[3] = Val::Auto;
    }
    let align_self = if centred { AlignSelf::Center } else { AlignSelf::Auto };
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
        align_self,
        background,
        anchor: None,
        translate: scope.get("transform").and_then(|value| parse_translate(&value)),
        animations: scope.get("animation").map(|value| parse_animation(&value)).unwrap_or_default(),
        animation_key: None,
    }
}

fn parse_translate(value: &str) -> Option<(Val, Val)> {
    let mut x = None;
    let mut y = None;
    let mut rest = value;
    while let Some(open) = rest.find('(') {
        let Some(close) = rest[open..].find(')').map(|end| open + end) else {
            break;
        };
        let name = rest[..open].trim();
        let args = rest[open + 1..close].split(',').map(str::trim).collect::<Vec<_>>();
        match name {
            "translate" => {
                x = args.first().and_then(|arg| parse_val(arg));
                y = args.get(1).and_then(|arg| parse_val(arg)).or(Some(Val::Px(0.0)));
            }
            "translateX" => x = args.first().and_then(|arg| parse_val(arg)),
            "translateY" => y = args.first().and_then(|arg| parse_val(arg)),
            _ => {}
        }
        rest = &rest[close + 1..];
    }
    (x.is_some() || y.is_some()).then(|| (x.unwrap_or(Val::Px(0.0)), y.unwrap_or(Val::Px(0.0))))
}

fn horizontal_margins_auto(scope: &Scope) -> bool {
    let margin = scope.get("margin").unwrap_or_default();
    let parts = margin.split_whitespace().collect::<Vec<_>>();
    let shorthand = match parts.len() {
        1 => parts[0] == "auto",
        2 | 3 => parts[1] == "auto",
        4 => parts[1] == "auto" && parts[3] == "auto",
        _ => false,
    };
    shorthand || (scope.get("margin-left").as_deref() == Some("auto") && scope.get("margin-right").as_deref() == Some("auto"))
}

fn is_covered(c: char) -> bool {
    (' '..='~').contains(&c)
}

pub fn uncovered_glyph_count(text: &str) -> usize {
    text.chars().filter(|c| *c != '\n' && !is_covered(*c)).count()
}

fn push_glyph(out: &mut String, c: char) {
    if c == BREAK || is_covered(c) {
        out.push(c);
        return;
    }
    out.push_str(match c {
        '·' | '•' => "|",
        '—' | '–' | '―' | '−' | '─' => "-",
        '═' => "=",
        '…' | '⋯' | '⁙' => "...",
        '→' | '➶' => "->",
        '▲' | '⟰' => "^",
        '▼' => "v",
        '◀' => "<",
        '≤' => "<=",
        '▶' | '▸' | '›' => ">",
        '±' => "+/-",
        '×' | '✕' => "x",
        '°' => " deg",
        '✓' => "+",
        '◆' | '◈' | '◎' | '●' | '✦' | '✶' | '✱' | '⁂' | '⁍' | '♨' | 'ϟ' | '⚔' | '⛨' => "*",
        _ => "?",
    });
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
            push_glyph(&mut collapsed, c);
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
    offset: (f32, f32),
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

    fn left(&self, value: f32) -> Val {
        self.x(value + self.offset.0)
    }

    fn top(&self, value: f32) -> Val {
        self.y(value + self.offset.1)
    }
}

fn canvas(attrs: &[(String, String)]) -> Canvas {
    let view: Vec<f32> = attr(attrs, "viewBox").unwrap_or("").split(|c: char| c == ' ' || c == ',').filter_map(|part| part.parse::<f32>().ok()).collect();
    match view.as_slice() {
        [_, _, width, height] if *width > 0.0 && *height > 0.0 => Canvas { size: Some((*width, *height)), offset: (0.0, 0.0) },
        _ => Canvas { size: None, offset: (0.0, 0.0) },
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

const MAX_USE_DEPTH: usize = 4;

struct SvgScope<'a> {
    ids: BTreeMap<&'a str, &'a Dom>,
    depth: usize,
}

impl<'a> SvgScope<'a> {
    fn new(nodes: &'a [Dom]) -> Self {
        let mut ids = BTreeMap::new();
        collect_ids(nodes, &mut ids);
        Self { ids, depth: 0 }
    }
}

fn collect_ids<'a>(nodes: &'a [Dom], ids: &mut BTreeMap<&'a str, &'a Dom>) {
    for node in nodes {
        let Dom::Element { attrs, children, .. } = node else { continue };
        if let Some(id) = attr(attrs, "id") {
            ids.insert(id, node);
        }
        collect_ids(children, ids);
    }
}

fn svg_shapes<'a>(nodes: &'a [Dom], canvas: Canvas, scope: &SvgScope<'a>, dropped: &mut BTreeMap<String, usize>) -> Vec<UiSpec> {
    let mut out = Vec::new();
    for node in nodes {
        let Dom::Element { tag, attrs, children } = node else { continue };
        let stroke_width = number(attrs, "stroke-width", 1.0);
        let stroke = paint(attrs, "stroke", None);
        let fill = paint(attrs, "fill", Some("black"));
        let shapes = match tag.as_str() {
            "circle" => {
                let (cx, cy, radius) = (number(attrs, "cx", 0.0), number(attrs, "cy", 0.0), number(attrs, "r", 0.0));
                vec![ShapeStyle {
                    left: canvas.left(cx - radius),
                    top: canvas.top(cy - radius),
                    width: canvas.x(2.0 * radius),
                    height: canvas.y(2.0 * radius),
                    rotation: 0.0,
                    round: true,
                    fill,
                    stroke,
                    stroke_width,
                }]
            }
            "rect" => vec![ShapeStyle {
                left: canvas.left(number(attrs, "x", 0.0)),
                top: canvas.top(number(attrs, "y", 0.0)),
                width: canvas.x(number(attrs, "width", 0.0)),
                height: canvas.y(number(attrs, "height", 0.0)),
                rotation: 0.0,
                round: false,
                fill,
                stroke,
                stroke_width,
            }],
            "line" => {
                let (x1, y1) = (number(attrs, "x1", 0.0), number(attrs, "y1", 0.0));
                let end = (number(attrs, "x2", 0.0), number(attrs, "y2", 0.0));
                vec![line_shape(canvas, (x1, y1), end, stroke_width, stroke)]
            }
            "polygon" => {
                let points = point_list(&attr(attrs, "points").unwrap_or("").replace(',', " "));
                polygon_shapes(canvas, &[(points, true)], fill, stroke, stroke_width)
            }
            "path" => match attr(attrs, "d").and_then(path_subpaths) {
                Some(subpaths) => polygon_shapes(canvas, &subpaths, fill, stroke, stroke_width),
                None => {
                    *dropped.entry(tag.clone()).or_insert(0) += 1;
                    continue;
                }
            },
            "g" => {
                out.extend(svg_shapes(children, canvas, scope, dropped));
                continue;
            }
            "defs" => continue,
            "use" => {
                let target = attr(attrs, "href")
                    .or_else(|| attr(attrs, "xlink:href"))
                    .and_then(|href| href.strip_prefix('#'))
                    .and_then(|id| scope.ids.get(id).copied());
                match target {
                    Some(target) if scope.depth < MAX_USE_DEPTH => {
                        let nested = Canvas {
                            size: canvas.size,
                            offset: (canvas.offset.0 + number(attrs, "x", 0.0), canvas.offset.1 + number(attrs, "y", 0.0)),
                        };
                        let nested_scope = SvgScope { ids: scope.ids.clone(), depth: scope.depth + 1 };
                        out.extend(svg_shapes(std::slice::from_ref(target), nested, &nested_scope, dropped));
                    }
                    _ => *dropped.entry(tag.clone()).or_insert(0) += 1,
                }
                continue;
            }
            other => {
                *dropped.entry(other.to_string()).or_insert(0) += 1;
                continue;
            }
        };
        out.extend(shapes.into_iter().map(|style| UiSpec::Shape { style }));
    }
    out
}

fn line_shape(canvas: Canvas, start: (f32, f32), end: (f32, f32), width: f32, color: Option<Color>) -> ShapeStyle {
    let (dx, dy) = (end.0 - start.0, end.1 - start.1);
    let length = (dx * dx + dy * dy).sqrt();
    let (mid_x, mid_y) = (start.0 + dx / 2.0, start.1 + dy / 2.0);
    ShapeStyle {
        left: canvas.left(mid_x - length / 2.0),
        top: canvas.top(mid_y - width / 2.0),
        width: canvas.x(length),
        height: canvas.y(width),
        rotation: dy.atan2(dx),
        round: false,
        fill: color,
        stroke: None,
        stroke_width: 0.0,
    }
}

fn point_list(text: &str) -> Vec<(f32, f32)> {
    let numbers: Vec<f32> = text.split_whitespace().filter_map(|part| part.parse::<f32>().ok()).collect();
    numbers.chunks_exact(2).map(|pair| (pair[0], pair[1])).collect()
}

fn polygon_shapes(canvas: Canvas, subpaths: &[(Vec<(f32, f32)>, bool)], fill: Option<Color>, stroke: Option<Color>, width: f32) -> Vec<ShapeStyle> {
    let mut shapes = Vec::new();
    if let Some(color) = fill {
        let closed: Vec<&Vec<(f32, f32)>> = subpaths.iter().filter(|(points, closed)| *closed && points.len() > 2).map(|(points, _)| points).collect();
        let ys = closed.iter().flat_map(|points| points.iter().map(|point| point.1));
        let (min_y, max_y) = ys.fold((f32::INFINITY, f32::NEG_INFINITY), |(low, high), y| (low.min(y), high.max(y)));
        if max_y > min_y {
            let rows = (max_y - min_y).ceil().clamp(1.0, 64.0) as usize;
            let step = (max_y - min_y) / rows as f32;
            for row in 0..rows {
                let top = min_y + row as f32 * step;
                let y = top + step / 2.0;
                let mut crossings: Vec<f32> = Vec::new();
                for points in &closed {
                    for index in 0..points.len() {
                        let (a, b) = (points[index], points[(index + 1) % points.len()]);
                        if (a.1 <= y && b.1 > y) || (b.1 <= y && a.1 > y) {
                            crossings.push(a.0 + (y - a.1) * (b.0 - a.0) / (b.1 - a.1));
                        }
                    }
                }
                crossings.sort_by(|a, b| a.total_cmp(b));
                for span in crossings.chunks_exact(2) {
                    shapes.push(ShapeStyle {
                        left: canvas.left(span[0]),
                        top: canvas.top(top),
                        width: canvas.x(span[1] - span[0]),
                        height: canvas.y(step),
                        rotation: 0.0,
                        round: false,
                        fill: Some(color),
                        stroke: None,
                        stroke_width: 0.0,
                    });
                }
            }
        }
    }
    if let Some(color) = stroke {
        for (points, closed) in subpaths {
            let count = points.len();
            let edges = if *closed { count } else { count.saturating_sub(1) };
            for index in 0..edges {
                shapes.push(line_shape(canvas, points[index], points[(index + 1) % count], width, Some(color)));
            }
        }
    }
    shapes
}

fn path_subpaths(d: &str) -> Option<Vec<(Vec<(f32, f32)>, bool)>> {
    let segments = path_segments(d)?;
    let mut subpaths: Vec<(Vec<(f32, f32)>, bool)> = Vec::new();
    let mut points: Vec<(f32, f32)> = Vec::new();
    let mut current = (0.0_f32, 0.0_f32);
    let mut start = (0.0_f32, 0.0_f32);
    let mut last_cubic: Option<(f32, f32)> = None;
    let mut last_quad: Option<(f32, f32)> = None;
    for (command, args) in &segments {
        let prev_cubic = last_cubic.take();
        let prev_quad = last_quad.take();
        let relative = command.is_ascii_lowercase();
        let base = |current: (f32, f32), x: f32, y: f32| if relative { (current.0 + x, current.1 + y) } else { (x, y) };
        match command.to_ascii_uppercase() {
            'M' => {
                for (index, pair) in args.chunks_exact(2).enumerate() {
                    let point = base(current, pair[0], pair[1]);
                    if index == 0 {
                        if points.len() > 1 {
                            subpaths.push((std::mem::take(&mut points), false));
                        }
                        points = vec![point];
                        start = point;
                    } else {
                        points.push(point);
                    }
                    current = point;
                }
            }
            'L' => {
                for pair in args.chunks_exact(2) {
                    let point = base(current, pair[0], pair[1]);
                    points.push(point);
                    current = point;
                }
            }
            'C' => {
                for chunk in args.chunks_exact(6) {
                    let first = base(current, chunk[0], chunk[1]);
                    let second = base(current, chunk[2], chunk[3]);
                    let end = base(current, chunk[4], chunk[5]);
                    points.extend(cubic_points(current, first, second, end));
                    current = end;
                    last_cubic = Some(second);
                }
            }
            'S' => {
                let mut reflected = prev_cubic;
                for chunk in args.chunks_exact(4) {
                    let first = reflected.map_or(current, |previous| (2.0 * current.0 - previous.0, 2.0 * current.1 - previous.1));
                    let second = base(current, chunk[0], chunk[1]);
                    let end = base(current, chunk[2], chunk[3]);
                    points.extend(cubic_points(current, first, second, end));
                    current = end;
                    reflected = Some(second);
                }
                last_cubic = reflected;
            }
            'Q' => {
                for chunk in args.chunks_exact(4) {
                    let control = base(current, chunk[0], chunk[1]);
                    let end = base(current, chunk[2], chunk[3]);
                    points.extend(quad_points(current, control, end));
                    current = end;
                    last_quad = Some(control);
                }
            }
            'T' => {
                let mut reflected = prev_quad;
                for chunk in args.chunks_exact(2) {
                    let control = reflected.map_or(current, |previous| (2.0 * current.0 - previous.0, 2.0 * current.1 - previous.1));
                    let end = base(current, chunk[0], chunk[1]);
                    points.extend(quad_points(current, control, end));
                    current = end;
                    reflected = Some(control);
                }
                last_quad = reflected;
            }
            'A' => {
                for chunk in args.chunks_exact(7) {
                    let end = base(current, chunk[5], chunk[6]);
                    points.extend(arc_points(current, (chunk[0], chunk[1]), chunk[2], chunk[3] != 0.0, chunk[4] != 0.0, end));
                    current = end;
                }
            }
            'H' => {
                for value in args {
                    let x = if relative { current.0 + value } else { *value };
                    current = (x, current.1);
                    points.push(current);
                }
            }
            'V' => {
                for value in args {
                    let y = if relative { current.1 + value } else { *value };
                    current = (current.0, y);
                    points.push(current);
                }
            }
            'Z' => {
                if points.len() > 1 {
                    subpaths.push((std::mem::take(&mut points), true));
                }
                points = vec![start];
                current = start;
            }
            _ => return None,
        }
    }
    if points.len() > 1 {
        subpaths.push((points, false));
    }
    (!subpaths.is_empty()).then_some(subpaths)
}

const CURVE_STEPS: usize = 12;

fn blend(weights: &[f32], points: &[(f32, f32)]) -> (f32, f32) {
    points.iter().zip(weights).fold((0.0, 0.0), |(x, y), (point, weight)| (x + weight * point.0, y + weight * point.1))
}

fn cubic_points(start: (f32, f32), first: (f32, f32), second: (f32, f32), end: (f32, f32)) -> Vec<(f32, f32)> {
    (1..=CURVE_STEPS)
        .map(|step| {
            let t = step as f32 / CURVE_STEPS as f32;
            let u = 1.0 - t;
            blend(&[u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t], &[start, first, second, end])
        })
        .collect()
}

fn quad_points(start: (f32, f32), control: (f32, f32), end: (f32, f32)) -> Vec<(f32, f32)> {
    (1..=CURVE_STEPS)
        .map(|step| {
            let t = step as f32 / CURVE_STEPS as f32;
            let u = 1.0 - t;
            blend(&[u * u, 2.0 * u * t, t * t], &[start, control, end])
        })
        .collect()
}

fn angle_between(u: (f32, f32), v: (f32, f32)) -> f32 {
    let dot = u.0 * v.0 + u.1 * v.1;
    let length = (u.0 * u.0 + u.1 * u.1).sqrt() * (v.0 * v.0 + v.1 * v.1).sqrt();
    let angle = (dot / length).clamp(-1.0, 1.0).acos();
    if u.0 * v.1 - u.1 * v.0 < 0.0 { -angle } else { angle }
}

fn arc_points(start: (f32, f32), radii: (f32, f32), rotation: f32, large: bool, sweep: bool, end: (f32, f32)) -> Vec<(f32, f32)> {
    if start == end {
        return Vec::new();
    }
    let (mut rx, mut ry) = (radii.0.abs(), radii.1.abs());
    if rx == 0.0 || ry == 0.0 {
        return vec![end];
    }
    let (sin, cos) = rotation.to_radians().sin_cos();
    let half = ((start.0 - end.0) / 2.0, (start.1 - end.1) / 2.0);
    let (x1, y1) = (cos * half.0 + sin * half.1, -sin * half.0 + cos * half.1);
    let lambda = x1 * x1 / (rx * rx) + y1 * y1 / (ry * ry);
    if lambda > 1.0 {
        rx *= lambda.sqrt();
        ry *= lambda.sqrt();
    }
    let (rx2, ry2) = (rx * rx, ry * ry);
    let numerator = rx2 * ry2 - rx2 * y1 * y1 - ry2 * x1 * x1;
    let denominator = rx2 * y1 * y1 + ry2 * x1 * x1;
    let root = if denominator == 0.0 { 0.0 } else { (numerator / denominator).max(0.0).sqrt() };
    let sign = if large == sweep { -1.0 } else { 1.0 };
    let (cx1, cy1) = (sign * root * rx * y1 / ry, sign * root * -ry * x1 / rx);
    let center = (cos * cx1 - sin * cy1 + (start.0 + end.0) / 2.0, sin * cx1 + cos * cy1 + (start.1 + end.1) / 2.0);
    let u = ((x1 - cx1) / rx, (y1 - cy1) / ry);
    let v = ((-x1 - cx1) / rx, (-y1 - cy1) / ry);
    let theta = angle_between((1.0, 0.0), u);
    let mut delta = angle_between(u, v);
    if !sweep && delta > 0.0 {
        delta -= std::f32::consts::TAU;
    }
    if sweep && delta < 0.0 {
        delta += std::f32::consts::TAU;
    }
    let steps = ((delta.abs() / (std::f32::consts::PI / 16.0)).ceil() as usize).max(4);
    (1..=steps)
        .map(|step| {
            let angle = theta + delta * step as f32 / steps as f32;
            let (sin_a, cos_a) = angle.sin_cos();
            (center.0 + rx * cos_a * cos - ry * sin_a * sin, center.1 + rx * cos_a * sin + ry * sin_a * cos)
        })
        .collect()
}

fn path_segments(d: &str) -> Option<Vec<(char, Vec<f32>)>> {
    let mut segments: Vec<(char, Vec<f32>)> = Vec::new();
    let bytes = d.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        let arc_flag = segments.last().is_some_and(|(command, args)| command.to_ascii_uppercase() == 'A' && matches!(args.len() % 7, 3 | 4));
        if arc_flag && (byte == b'0' || byte == b'1') {
            segments.last_mut()?.1.push(f32::from(byte - b'0'));
            index += 1;
        } else if byte.is_ascii_alphabetic() {
            segments.push((byte as char, Vec::new()));
            index += 1;
        } else if byte.is_ascii_whitespace() || byte == b',' {
            index += 1;
        } else {
            let start = index;
            if byte == b'-' || byte == b'+' {
                index += 1;
            }
            while index < bytes.len() && (bytes[index].is_ascii_digit() || bytes[index] == b'.') {
                index += 1;
            }
            if index < bytes.len() && (bytes[index] == b'e' || bytes[index] == b'E') {
                index += 1;
                if index < bytes.len() && (bytes[index] == b'-' || bytes[index] == b'+') {
                    index += 1;
                }
                while index < bytes.len() && bytes[index].is_ascii_digit() {
                    index += 1;
                }
            }
            let value: f32 = d.get(start..index)?.parse().ok()?;
            segments.last_mut()?.1.push(value);
        }
    }
    Some(segments)
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
                if scope.props.is_empty() && INLINE.contains(&tag.as_str()) && attr(attrs, "data-world-anchor").is_none() {
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
                    style.animation_key = attr(attrs, "id").map(str::to_string);
                    if let Some(id) = attr(attrs, "data-world-anchor") {
                        style.anchor = Some(WorldAnchor { id: id.to_string(), offset: anchor_offset(attr(attrs, "data-anchor-offset")) });
                        style.inset = Some([Val::Px(0.0), Val::Auto, Val::Auto, Val::Px(0.0)]);
                    }
                    let inner = if tag == "svg" {
                        fit_svg(&mut style, attrs);
                        let scope = SvgScope::new(children);
                        svg_shapes(children, canvas(attrs), &scope, dropped)
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

    fn box_styles(spec: &UiSpec, out: &mut Vec<BoxStyle>) {
        if let UiSpec::Box { style, children } = spec {
            out.push(style.clone());
            for child in children {
                box_styles(child, out);
            }
        }
    }

    #[test]
    fn max_content_boxes_with_auto_margins_shrink_and_centre() {
        let built = build(r#"<div style="position:absolute;top:24%;left:0;right:0px;margin:0 auto;width:max-content">DOWNTOWN</div>"#);
        let mut styles = Vec::new();
        box_styles(&built.spec, &mut styles);
        let banner = styles.iter().find(|style| style.align_self == AlignSelf::Center).expect("centred banner box");
        let inset = banner.inset.clone().expect("absolute");
        assert!(matches!(inset, [Val::Percent(top), Val::Auto, Val::Auto, Val::Auto] if top == 24.0));
        let plain = build(r#"<div style="position:absolute;left:0;right:0">wide</div>"#);
        let mut plain_styles = Vec::new();
        box_styles(&plain.spec, &mut plain_styles);
        assert!(plain_styles.iter().all(|style| style.align_self == AlignSelf::Auto));
    }

    #[test]
    fn animated_boxes_keep_their_keyframes_under_their_id() {
        let built = build(r#"<div id="dzNEXUS" style="position:absolute;animation:dzin 4s forwards">NEXUS</div><div style="animation:missing 1s">x</div><div><p style="animation:dzin 1s">y</p></div><style>@keyframes dzin{0%{opacity:0}100%{opacity:0;visibility:hidden}}</style>"#);
        let keys = built.spec.animation_keys();
        assert_eq!(keys.len(), 2);
        assert_eq!(keys[0], "dzNEXUS");
        assert!(keys[1].starts_with("path:") && keys[1].ends_with(":dzin"));
        let mut styles = Vec::new();
        box_styles(&built.spec, &mut styles);
        let banner = styles.iter().find(|style| style.animation_key.as_deref() == Some("dzNEXUS")).expect("banner");
        assert_eq!(sample(&banner.animations, 9.0), crate::css_anim::Sampled { opacity: 0.0, visible: false });
        assert!(styles.iter().all(|style| style.animations.iter().all(|animation| animation.keyframes.is_some())));
    }

    #[test]
    fn translate_transforms_map_to_node_relative_offsets() {
        assert_eq!(parse_translate("translateX(-50%)"), Some((Val::Percent(-50.0), Val::Px(0.0))));
        assert_eq!(parse_translate("translate(-50%,-190%)"), Some((Val::Percent(-50.0), Val::Percent(-190.0))));
        assert_eq!(parse_translate("translate(4px)"), Some((Val::Px(4.0), Val::Px(0.0))));
        assert_eq!(parse_translate("scale(2) translateY(-3px)"), Some((Val::Px(0.0), Val::Px(-3.0))));
        assert_eq!(parse_translate("scale(2)"), None);
        let built = build(r#"<div style="position:absolute;left:50%;transform:translateX(-50%)">c</div>"#);
        let mut styles = Vec::new();
        box_styles(&built.spec, &mut styles);
        assert!(styles.iter().any(|style| style.translate == Some((Val::Percent(-50.0), Val::Px(0.0)))));
    }

    #[test]
    fn world_anchored_elements_become_anchored_absolute_boxes() {
        let built = build(r#"<div data-world-anchor="district-a" data-anchor-offset="0 0.4 0"><div>tag</div></div><div data-world-anchor="foe"><b>x</b></div><div>y</div>"#);
        assert!(built.warnings.is_empty());
        assert_eq!(built.spec.anchors(), 2);
        let mut styles = Vec::new();
        box_styles(&built.spec, &mut styles);
        let anchored = styles.iter().filter_map(|style| style.anchor.clone().map(|anchor| (anchor, style.inset.clone()))).collect::<Vec<_>>();
        assert_eq!(anchored[0].0, WorldAnchor { id: "district-a".to_string(), offset: Vec3::new(0.0, 0.4, 0.0) });
        assert_eq!(anchored[1].0.offset, Vec3::ZERO);
        assert!(anchored.iter().all(|(_, inset)| matches!(inset, Some([Val::Px(_), Val::Auto, Val::Auto, Val::Px(_)]))));
    }

    #[test]
    fn svg_shapes_map_to_absolute_nodes() {
        let built = build(r##"<svg width="10" height="10" viewBox="0 0 10 10"><circle cx="5" cy="5" r="4" fill="none" stroke="#ffffff"/><line x1="0" y1="0" x2="10" y2="0" stroke="#ff0000"/><rect x="1" y="1" width="2" height="2" fill="#00ff00"/></svg>"##);
        assert!(built.warnings.is_empty());
        assert_eq!(built.spec.count(), 5);
    }

    #[test]
    fn oklch_colors_convert_to_srgb() {
        let white = parse_color("oklch(1 0 0)").expect("white").to_srgba();
        assert!((white.red - 1.0).abs() < 0.01 && (white.green - 1.0).abs() < 0.01 && (white.blue - 1.0).abs() < 0.01);
        assert!(parse_color("oklch(0.78 0.17 200 / .55)").is_some());
        assert!(parse_color("oklch(bogus)").is_none());
    }

    #[test]
    fn svg_polygon_and_path_are_drawn_not_dropped() {
        let built = build(r##"<svg viewBox="0 0 10 10"><polygon points="0,0 10,0 5,10" fill="oklch(0.6 0.12 200)" fill-opacity=".16" stroke="#00ff00"/><path d="M5 1l2 2h-4z" fill="#ff0000"/></svg>"##);
        assert!(built.warnings.is_empty());
        assert!(built.spec.count() > 6);
    }

    #[test]
    fn svg_path_with_curves_is_drawn() {
        for d in ["M1 1 C2 0 4 0 5 1 Z", "M1 1 Q3 0 5 1 Z", "M1 1 A2 2 0 0 1 5 1 Z", "M1 1 S3 0 5 1 T7 1 Z"] {
            let built = build(&format!(r##"<svg viewBox="0 0 10 10"><path d="{d}" fill="#ff0000"/></svg>"##));
            assert!(built.warnings.is_empty(), "{d}");
            assert!(built.spec.count() > 2, "{d}");
        }
    }

    #[test]
    fn compact_arc_flags_parse_as_single_digits() {
        let compact = path_segments("M0 0a1 1 0 011 1a1 1 0 100 1").expect("compact");
        let spaced = path_segments("M0 0 a1 1 0 0 1 1 1 a1 1 0 1 0 0 1").expect("spaced");
        assert_eq!(compact, spaced);
        assert_eq!(compact[1], ('a', vec![1.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0]));
        assert_eq!(compact[2], ('a', vec![1.0, 1.0, 0.0, 1.0, 0.0, 0.0, 1.0]));
        assert_eq!(path_subpaths("M0 0a1 1 0 011 1").expect("compact arc"), path_subpaths("M0 0 a1 1 0 0 1 1 1").expect("spaced arc"));
    }

    #[test]
    fn svg_defs_are_not_drawn_and_use_draws_them_translated() {
        let built = build(r##"<svg viewBox="0 0 4 4"><defs><circle id="dot" cx="0" cy="0" r="1" fill="#ff0000"/></defs><use href="#dot" x="2" y="1"/></svg>"##);
        assert!(built.warnings.is_empty());
        let UiSpec::Box { children, .. } = &built.spec else { panic!("root is a box") };
        let UiSpec::Box { children, .. } = &children[0] else { panic!("svg is a box") };
        assert_eq!(children.len(), 1);
        let UiSpec::Shape { style } = &children[0] else { panic!("use draws one shape") };
        assert_eq!(style.left, Val::Percent(25.0));
        assert_eq!(style.top, Val::Percent(0.0));
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
    fn glyph_fallback_keeps_text_inside_the_font() {
        let sample = "·•—–―─═…⋯⁙×→➶⟰▲▼◀▶▸›≤±✕✓°−◆◈◎●✦✶✱⁂⁍♨ϟ⚔⛨";
        for c in sample.chars() {
            let mut out = String::new();
            push_glyph(&mut out, c);
            assert!(!out.is_empty() && uncovered_glyph_count(&out) == 0, "{c:?} maps to {out:?}");
        }
        let mut out = String::new();
        push_glyph(&mut out, 'é');
        assert_eq!(out, "?");
        assert_eq!(uncovered_glyph_count("a | b\n"), 0);
        assert_eq!(uncovered_glyph_count("a\u{e9}"), 1);
    }

    #[test]
    fn calc_insets_resolve_to_the_summed_terms() {
        assert!(matches!(parse_val("calc(18px + 0px)"), Some(Val::Px(v)) if v == 18.0));
        assert!(matches!(parse_val("calc(24px - 4px)"), Some(Val::Px(v)) if v == 20.0));
        assert!(matches!(parse_val("calc(-6px + 2px)"), Some(Val::Px(v)) if v == -4.0));
        assert!(matches!(parse_val("calc(50% + 0px)"), Some(Val::Percent(v)) if v == 50.0));
        assert!(matches!(parse_val("calc(24% + 200px)"), Some(Val::Percent(v)) if v == 24.0));
        assert!(parse_val("calc(oops)").is_none());
    }

    #[test]
    fn overlapping_pairs_counts_only_real_intersections() {
        let touching = [[0.0, 0.0, 10.0, 10.0], [10.0, 0.0, 20.0, 10.0]];
        assert!(overlapping_pairs(&touching).is_empty());
        let overlap = [[0.0, 0.0, 10.0, 10.0], [5.0, 5.0, 15.0, 15.0], [40.0, 0.0, 50.0, 10.0]];
        assert_eq!(overlapping_pairs(&overlap), vec![(0, 1)]);
        let stacked = [[0.0, 0.0, 10.0, 10.0], [0.0, 0.0, 10.0, 10.0], [0.0, 0.0, 10.0, 10.0]];
        assert_eq!(overlapping_pairs(&stacked).len(), 3);
        assert!(overlapping_pairs(&[]).is_empty());
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

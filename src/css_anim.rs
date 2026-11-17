use std::collections::BTreeMap;
use std::sync::Arc;

const EASE: Timing = Timing::Bezier(0.25, 0.1, 0.25, 1.0);

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Keyframes {
    frames: Vec<(f32, BTreeMap<String, String>)>,
}

impl Keyframes {
    pub fn specifies(&self, property: &str) -> bool {
        self.frames.iter().any(|(_, declarations)| declarations.contains_key(property))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Timing {
    Linear,
    Bezier(f32, f32, f32, f32),
    Steps(u32, bool),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fill {
    None,
    Forwards,
    Backwards,
    Both,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Normal,
    Reverse,
    Alternate,
    AlternateReverse,
}

fn parse_direction(token: &str) -> Option<Direction> {
    match token {
        "normal" => Some(Direction::Normal),
        "reverse" => Some(Direction::Reverse),
        "alternate" => Some(Direction::Alternate),
        "alternate-reverse" => Some(Direction::AlternateReverse),
        _ => None,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Animation {
    pub name: String,
    pub duration: f32,
    pub delay: f32,
    pub timing: Timing,
    pub iterations: f32,
    pub fill: Fill,
    pub direction: Direction,
    pub paused: bool,
    pub keyframes: Option<Arc<Keyframes>>,
}

impl Animation {
    fn named(name: &str) -> Self {
        Self { name: name.to_string(), duration: 0.0, delay: 0.0, timing: EASE, iterations: 1.0, fill: Fill::None, direction: Direction::Normal, paused: false, keyframes: None }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Transition {
    pub property: String,
    pub duration: f32,
    pub delay: f32,
    pub timing: Timing,
}

impl Transition {
    pub fn covers(&self, property: &str) -> bool {
        self.property == property || self.property == "all"
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Track<T: Copy> {
    pub from: T,
    pub to: T,
    pub start: f32,
    pub duration: f32,
    pub delay: f32,
    pub timing: Timing,
    pub reversing_start: T,
    pub shortening: f32,
}

impl<T: Copy + PartialEq> Track<T> {
    pub fn settled(value: T) -> Self {
        Self { from: value, to: value, start: 0.0, duration: 0.0, delay: 0.0, timing: Timing::Linear, reversing_start: value, shortening: 1.0 }
    }

    pub fn active(&self, now: f32) -> bool {
        (self.duration > 0.0 || self.delay > 0.0) && now < self.start + self.delay + self.duration
    }

    pub fn retarget(&self, now: f32, current: T, target: T, transition: Option<&Transition>) -> (Self, bool) {
        let Some(transition) = transition else {
            return (Self::settled(target), false);
        };
        if current == target {
            return (Self::settled(target), false);
        }
        if self.active(now) && target == self.reversing_start {
            let output = self.eased(now);
            let factor = (output * self.shortening + (1.0 - self.shortening)).abs().clamp(0.0, 1.0);
            let delay = if transition.delay < 0.0 { transition.delay * factor } else { transition.delay };
            let track = Self { from: current, to: target, start: now, duration: transition.duration * factor, delay, timing: transition.timing, reversing_start: self.to, shortening: factor };
            return (track, true);
        }
        let track = Self { from: current, to: target, start: now, duration: transition.duration, delay: transition.delay, timing: transition.timing, reversing_start: current, shortening: 1.0 };
        (track, false)
    }

    pub fn eased(&self, now: f32) -> f32 {
        if self.duration <= 0.0 {
            return 1.0;
        }
        let local = (now - self.start - self.delay) / self.duration;
        if local <= 0.0 {
            0.0
        } else if local >= 1.0 {
            1.0
        } else {
            self.timing.apply(local)
        }
    }
}

impl Track<f32> {
    pub fn value(&self, now: f32) -> f32 {
        let eased = self.eased(now);
        self.from + (self.to - self.from) * eased
    }
}

impl Track<bool> {
    pub fn value(&self, now: f32) -> bool {
        let eased = self.eased(now);
        if self.from == self.to || eased <= 0.0 {
            self.from
        } else if eased >= 1.0 {
            self.to
        } else {
            self.from || self.to
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Length {
    pub px: f32,
    pub percent: f32,
}

impl Length {
    fn lerp(self, other: Self, t: f32) -> Self {
        Self { px: self.px + (other.px - self.px) * t, percent: self.percent + (other.percent - self.percent) * t }
    }

    pub fn resolve(self, size: f32) -> f32 {
        self.px + self.percent * size / 100.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TransformFunction {
    Translate(Length, Length),
    Scale(f32, f32),
    Rotate(f32),
}

impl TransformFunction {
    fn kind(&self) -> u8 {
        match self {
            Self::Translate(..) => 0,
            Self::Scale(..) => 1,
            Self::Rotate(..) => 2,
        }
    }

    fn identity(&self) -> Self {
        match self {
            Self::Translate(..) => Self::Translate(Length::default(), Length::default()),
            Self::Scale(..) => Self::Scale(1.0, 1.0),
            Self::Rotate(..) => Self::Rotate(0.0),
        }
    }

    fn lerp(&self, other: &Self, t: f32) -> Self {
        match (self, other) {
            (Self::Translate(ax, ay), Self::Translate(bx, by)) => Self::Translate(ax.lerp(*bx, t), ay.lerp(*by, t)),
            (Self::Scale(ax, ay), Self::Scale(bx, by)) => Self::Scale(ax + (bx - ax) * t, ay + (by - ay) * t),
            (Self::Rotate(a), Self::Rotate(b)) => Self::Rotate(a + (b - a) * t),
            _ => *self,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Resolved {
    pub tx: Length,
    pub ty: Length,
    pub sx: f32,
    pub sy: f32,
    pub rotation: f32,
}

pub fn compose(functions: &[TransformFunction]) -> Resolved {
    let mut out = Resolved { sx: 1.0, sy: 1.0, ..Default::default() };
    for function in functions {
        match function {
            TransformFunction::Translate(x, y) => {
                out.tx = Length { px: out.tx.px + x.px, percent: out.tx.percent + x.percent };
                out.ty = Length { px: out.ty.px + y.px, percent: out.ty.percent + y.percent };
            }
            TransformFunction::Scale(x, y) => {
                out.sx *= x;
                out.sy *= y;
            }
            TransformFunction::Rotate(angle) => out.rotation += angle,
        }
    }
    out
}

fn canonical(functions: &[TransformFunction]) -> Vec<TransformFunction> {
    let resolved = compose(functions);
    vec![TransformFunction::Translate(resolved.tx, resolved.ty), TransformFunction::Rotate(resolved.rotation), TransformFunction::Scale(resolved.sx, resolved.sy)]
}

pub fn interpolate_transform(a: &[TransformFunction], b: &[TransformFunction], t: f32) -> Vec<TransformFunction> {
    let a = if a.is_empty() { b.iter().map(TransformFunction::identity).collect() } else { a.to_vec() };
    let b = if b.is_empty() { a.iter().map(TransformFunction::identity).collect() } else { b.to_vec() };
    let (a, b) = if a.len() == b.len() && a.iter().zip(&b).all(|(x, y)| x.kind() == y.kind()) { (a, b) } else { (canonical(&a), canonical(&b)) };
    a.iter().zip(&b).map(|(x, y)| x.lerp(y, t)).collect()
}

fn parse_length(token: &str) -> Option<Length> {
    let token = token.trim();
    if let Some(percent) = token.strip_suffix('%') {
        return percent.trim().parse::<f32>().ok().map(|percent| Length { px: 0.0, percent });
    }
    let number = token.strip_suffix("px").unwrap_or(token);
    number.trim().parse::<f32>().ok().map(|px| Length { px, percent: 0.0 })
}

fn parse_angle(token: &str) -> Option<f32> {
    let token = token.trim();
    if let Some(value) = token.strip_suffix("deg") {
        return value.parse::<f32>().ok().map(f32::to_radians);
    }
    if let Some(value) = token.strip_suffix("grad") {
        return value.parse::<f32>().ok().map(|value| value * std::f32::consts::PI / 200.0);
    }
    if let Some(value) = token.strip_suffix("rad") {
        return value.parse::<f32>().ok();
    }
    if let Some(value) = token.strip_suffix("turn") {
        return value.parse::<f32>().ok().map(|value| value * std::f32::consts::TAU);
    }
    (token == "0").then_some(0.0)
}

pub fn parse_transform(value: &str) -> Option<Vec<TransformFunction>> {
    let value = value.trim();
    if value == "none" || value.is_empty() {
        return Some(Vec::new());
    }
    let mut out = Vec::new();
    let mut rest = value;
    while let Some(open) = rest.find('(') {
        let close = open + rest[open..].find(')')?;
        let name = rest[..open].trim();
        let args = rest[open + 1..close].split(',').map(str::trim).collect::<Vec<_>>();
        let zero = Length::default();
        out.push(match name {
            "translate" => TransformFunction::Translate(parse_length(args.first()?)?, args.get(1).map(|arg| parse_length(arg)).unwrap_or(Some(zero))?),
            "translateX" => TransformFunction::Translate(parse_length(args.first()?)?, zero),
            "translateY" => TransformFunction::Translate(zero, parse_length(args.first()?)?),
            "scale" => {
                let x = args.first()?.parse::<f32>().ok()?;
                TransformFunction::Scale(x, args.get(1).map(|arg| arg.parse::<f32>().ok()).unwrap_or(Some(x))?)
            }
            "scaleX" => TransformFunction::Scale(args.first()?.parse::<f32>().ok()?, 1.0),
            "scaleY" => TransformFunction::Scale(1.0, args.first()?.parse::<f32>().ok()?),
            "rotate" | "rotateZ" => TransformFunction::Rotate(parse_angle(args.first()?)?),
            _ => return None,
        });
        rest = &rest[close + 1..];
    }
    rest.trim().is_empty().then_some(out)
}

pub type Rgba = [f32; 4];

pub fn interpolate_color(a: Rgba, b: Rgba, t: f32) -> Rgba {
    let alpha = a[3] + (b[3] - a[3]) * t;
    if alpha <= 0.0 {
        return [0.0, 0.0, 0.0, 0.0];
    }
    let channel = |index: usize| ((a[index] * a[3]) + ((b[index] * b[3]) - (a[index] * a[3])) * t) / alpha;
    [channel(0), channel(1), channel(2), alpha]
}

pub fn parse_rgba(value: &str) -> Option<Rgba> {
    let color = crate::ui_html::parse_color(value)?.to_srgba();
    Some([color.red, color.green, color.blue, color.alpha])
}

#[derive(Clone, Debug, PartialEq)]
pub struct Sampled {
    pub opacity: f32,
    pub visible: bool,
    pub color: Option<Rgba>,
    pub background: Option<Rgba>,
    pub transform: Option<Vec<TransformFunction>>,
}

impl Default for Sampled {
    fn default() -> Self {
        Self { opacity: 1.0, visible: true, color: None, background: None, transform: None }
    }
}

fn block_end(text: &str) -> Option<usize> {
    let mut depth = 0i32;
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

fn parse_declarations(body: &str) -> BTreeMap<String, String> {
    body.split(';')
        .filter_map(|piece| {
            let (name, value) = piece.split_once(':')?;
            let value = value.trim().trim_end_matches("!important").trim();
            Some((name.trim().to_ascii_lowercase(), value.to_string()))
        })
        .filter(|(name, value)| !name.is_empty() && !value.is_empty())
        .collect()
}

fn keyframe_offset(selector: &str) -> Option<f32> {
    match selector.trim() {
        "from" => Some(0.0),
        "to" => Some(1.0),
        other => other.strip_suffix('%')?.trim().parse::<f32>().ok().map(|percent| (percent / 100.0).clamp(0.0, 1.0)),
    }
}

pub fn parse_keyframes(css: &str) -> BTreeMap<String, Arc<Keyframes>> {
    let mut out = BTreeMap::new();
    let mut rest = css;
    while let Some(at) = rest.find("@keyframes") {
        let after = &rest[at + "@keyframes".len()..];
        let Some(open) = after.find('{') else { break };
        let name = after[..open].trim().to_string();
        let Some(close) = block_end(&after[open..]) else { break };
        let body = &after[open + 1..open + close];
        rest = &after[open + close + 1..];
        let mut frames = Vec::new();
        let mut inner = body;
        while let Some(brace) = inner.find('{') {
            let selectors = &inner[..brace];
            let Some(end) = inner[brace..].find('}').map(|end| brace + end) else { break };
            let declarations = parse_declarations(&inner[brace + 1..end]);
            for offset in selectors.split(',').filter_map(keyframe_offset) {
                frames.push((offset, declarations.clone()));
            }
            inner = &inner[end + 1..];
        }
        frames.sort_by(|a, b| a.0.total_cmp(&b.0));
        if !name.is_empty() {
            out.insert(name, Arc::new(Keyframes { frames }));
        }
    }
    out
}

fn split_top_level(value: &str, separator: char) -> Vec<String> {
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut current = String::new();
    for c in value.chars() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            _ => {}
        }
        let splits = if separator == ' ' { c.is_whitespace() } else { c == separator };
        if splits && depth == 0 {
            if !current.trim().is_empty() {
                parts.push(current.trim().to_string());
            }
            current.clear();
        } else {
            current.push(c);
        }
    }
    if !current.trim().is_empty() {
        parts.push(current.trim().to_string());
    }
    parts
}

fn parse_time(token: &str) -> Option<f32> {
    if let Some(ms) = token.strip_suffix("ms") {
        return ms.parse::<f32>().ok().map(|value| value / 1000.0);
    }
    token.strip_suffix('s')?.parse::<f32>().ok()
}

fn parse_timing(token: &str) -> Option<Timing> {
    match token {
        "linear" => Some(Timing::Linear),
        "ease" => Some(EASE),
        "ease-in" => Some(Timing::Bezier(0.42, 0.0, 1.0, 1.0)),
        "ease-out" => Some(Timing::Bezier(0.0, 0.0, 0.58, 1.0)),
        "ease-in-out" => Some(Timing::Bezier(0.42, 0.0, 0.58, 1.0)),
        "step-start" => Some(Timing::Steps(1, true)),
        "step-end" => Some(Timing::Steps(1, false)),
        _ => {
            let (function, args) = token.split_once('(')?;
            let args = args.strip_suffix(')')?.split(',').map(str::trim).collect::<Vec<_>>();
            match function {
                "cubic-bezier" => {
                    let numbers = args.iter().map(|arg| arg.parse::<f32>().ok()).collect::<Option<Vec<_>>>()?;
                    match numbers.as_slice() {
                        [x1, y1, x2, y2] => Some(Timing::Bezier(*x1, *y1, *x2, *y2)),
                        _ => None,
                    }
                }
                "steps" => {
                    let count = args.first()?.parse::<u32>().ok()?.max(1);
                    let start = matches!(args.get(1).copied(), Some("start" | "jump-start"));
                    Some(Timing::Steps(count, start))
                }
                _ => None,
            }
        }
    }
}

pub fn parse_animation(value: &str) -> Vec<Animation> {
    split_top_level(value, ',')
        .iter()
        .filter_map(|single| {
            let mut name = None;
            let mut times = Vec::new();
            let mut timing = EASE;
            let mut iterations = 1.0;
            let mut fill = Fill::None;
            let mut paused = false;
            let mut direction = Direction::Normal;
            for token in split_top_level(single, ' ') {
                if let Some(time) = parse_time(&token) {
                    times.push(time);
                } else if let Some(parsed) = parse_timing(&token) {
                    timing = parsed;
                } else if token == "infinite" {
                    iterations = f32::INFINITY;
                } else if let Ok(count) = token.parse::<f32>() {
                    iterations = count.max(0.0);
                } else if let Some(parsed) = match token.as_str() {
                    "forwards" => Some(Fill::Forwards),
                    "backwards" => Some(Fill::Backwards),
                    "both" => Some(Fill::Both),
                    _ => None,
                } {
                    fill = parsed;
                } else if token == "paused" {
                    paused = true;
                } else if let Some(parsed) = parse_direction(&token) {
                    direction = parsed;
                } else if matches!(token.as_str(), "none" | "running") {
                    if token == "none" && name.is_none() {
                        name = Some(String::new());
                    }
                } else if name.is_none() {
                    name = Some(token.clone());
                }
            }
            let name = name.filter(|name| !name.is_empty())?;
            Some(Animation {
                name,
                duration: times.first().copied().unwrap_or(0.0),
                delay: times.get(1).copied().unwrap_or(0.0),
                timing,
                iterations,
                fill,
                direction,
                paused,
                keyframes: None,
            })
        })
        .collect()
}

fn list(value: Option<String>) -> Vec<String> {
    value.map(|value| split_top_level(&value, ',')).unwrap_or_default()
}

fn cycled<T: Clone>(values: &[T], index: usize) -> Option<T> {
    (!values.is_empty()).then(|| values[index % values.len()].clone())
}

pub fn apply_animation_longhands(shorthand: Vec<Animation>, get: impl Fn(&str) -> Option<String>) -> Vec<Animation> {
    let names = list(get("animation-name"));
    let mut animations = if names.is_empty() {
        shorthand
    } else {
        names
            .iter()
            .enumerate()
            .filter(|(_, name)| name.as_str() != "none")
            .map(|(index, name)| {
                let mut animation = shorthand.get(index).cloned().unwrap_or_else(|| Animation::named(name));
                animation.name = name.clone();
                animation
            })
            .collect()
    };
    let durations = list(get("animation-duration")).iter().filter_map(|value| parse_time(value)).collect::<Vec<_>>();
    let delays = list(get("animation-delay")).iter().filter_map(|value| parse_time(value)).collect::<Vec<_>>();
    let timings = list(get("animation-timing-function")).iter().filter_map(|value| parse_timing(value)).collect::<Vec<_>>();
    let counts = list(get("animation-iteration-count"))
        .iter()
        .filter_map(|value| if value == "infinite" { Some(f32::INFINITY) } else { value.parse::<f32>().ok() })
        .collect::<Vec<_>>();
    let fills = list(get("animation-fill-mode"))
        .iter()
        .map(|value| match value.as_str() {
            "forwards" => Fill::Forwards,
            "backwards" => Fill::Backwards,
            "both" => Fill::Both,
            _ => Fill::None,
        })
        .collect::<Vec<_>>();
    let states = list(get("animation-play-state")).iter().map(|value| value == "paused").collect::<Vec<_>>();
    let directions = list(get("animation-direction")).iter().filter_map(|value| parse_direction(value)).collect::<Vec<_>>();
    for (index, animation) in animations.iter_mut().enumerate() {
        if let Some(value) = cycled(&durations, index) {
            animation.duration = value;
        }
        if let Some(value) = cycled(&delays, index) {
            animation.delay = value;
        }
        if let Some(value) = cycled(&timings, index) {
            animation.timing = value;
        }
        if let Some(value) = cycled(&counts, index) {
            animation.iterations = value.max(0.0);
        }
        if let Some(value) = cycled(&fills, index) {
            animation.fill = value;
        }
        if let Some(value) = cycled(&states, index) {
            animation.paused = value;
        }
        if let Some(value) = cycled(&directions, index) {
            animation.direction = value;
        }
    }
    animations
}

pub fn parse_transitions(get: impl Fn(&str) -> Option<String>) -> Vec<Transition> {
    let mut transitions = list(get("transition"))
        .iter()
        .filter_map(|single| {
            let mut property = None;
            let mut times = Vec::new();
            let mut timing = EASE;
            for token in split_top_level(single, ' ') {
                if let Some(time) = parse_time(&token) {
                    times.push(time);
                } else if let Some(parsed) = parse_timing(&token) {
                    timing = parsed;
                } else if property.is_none() {
                    property = Some(token.to_ascii_lowercase());
                }
            }
            let property = property.unwrap_or_else(|| "all".to_string());
            (property != "none").then(|| Transition { property, duration: times.first().copied().unwrap_or(0.0), delay: times.get(1).copied().unwrap_or(0.0), timing })
        })
        .collect::<Vec<_>>();
    let properties = list(get("transition-property"));
    if !properties.is_empty() {
        transitions = properties
            .iter()
            .enumerate()
            .filter(|(_, property)| property.as_str() != "none")
            .map(|(index, property)| {
                let mut transition = transitions.get(index).cloned().unwrap_or(Transition { property: String::new(), duration: 0.0, delay: 0.0, timing: EASE });
                transition.property = property.to_ascii_lowercase();
                transition
            })
            .collect();
    }
    let durations = list(get("transition-duration")).iter().filter_map(|value| parse_time(value)).collect::<Vec<_>>();
    let delays = list(get("transition-delay")).iter().filter_map(|value| parse_time(value)).collect::<Vec<_>>();
    let timings = list(get("transition-timing-function")).iter().filter_map(|value| parse_timing(value)).collect::<Vec<_>>();
    for (index, transition) in transitions.iter_mut().enumerate() {
        if let Some(value) = cycled(&durations, index) {
            transition.duration = value;
        }
        if let Some(value) = cycled(&delays, index) {
            transition.delay = value;
        }
        if let Some(value) = cycled(&timings, index) {
            transition.timing = value;
        }
    }
    transitions.retain(|transition| transition.duration > 0.0 || transition.delay > 0.0);
    transitions
}

pub fn transition_for<'a>(transitions: &'a [Transition], property: &str) -> Option<&'a Transition> {
    transitions.iter().rev().find(|transition| transition.covers(property))
}

fn bezier(p1: f32, p2: f32, t: f32) -> f32 {
    let u = 1.0 - t;
    3.0 * u * u * t * p1 + 3.0 * u * t * t * p2 + t * t * t
}

fn bezier_slope(p1: f32, p2: f32, t: f32) -> f32 {
    let u = 1.0 - t;
    3.0 * u * u * p1 + 6.0 * u * t * (p2 - p1) + 3.0 * t * t * (1.0 - p2)
}

impl Timing {
    pub fn apply(self, x: f32) -> f32 {
        let x = x.clamp(0.0, 1.0);
        match self {
            Timing::Linear => x,
            Timing::Steps(count, start) => {
                let steps = count as f32;
                let step = if start { (x * steps).floor() + 1.0 } else { (x * steps).floor() };
                (step / steps).clamp(0.0, 1.0)
            }
            Timing::Bezier(x1, y1, x2, y2) => {
                let mut t = x;
                for _ in 0..8 {
                    let error = bezier(x1, x2, t) - x;
                    let slope = bezier_slope(x1, x2, t);
                    if error.abs() < 1e-6 || slope.abs() < 1e-6 {
                        break;
                    }
                    t = (t - error / slope).clamp(0.0, 1.0);
                }
                if (bezier(x1, x2, t) - x).abs() > 1e-4 {
                    let (mut low, mut high) = (0.0f32, 1.0f32);
                    for _ in 0..40 {
                        t = (low + high) * 0.5;
                        if bezier(x1, x2, t) < x {
                            low = t;
                        } else {
                            high = t;
                        }
                    }
                }
                bezier(y1, y2, t)
            }
        }
    }
}

fn directed(direction: Direction, iteration: f32, p: f32) -> f32 {
    let odd = (iteration as i64).rem_euclid(2) == 1;
    match direction {
        Direction::Normal => p,
        Direction::Reverse => 1.0 - p,
        Direction::Alternate => if odd { 1.0 - p } else { p },
        Direction::AlternateReverse => if odd { p } else { 1.0 - p },
    }
}

fn progress(animation: &Animation, elapsed: f32) -> Option<f32> {
    let local = elapsed - animation.delay;
    let backwards = matches!(animation.fill, Fill::Backwards | Fill::Both);
    let forwards = matches!(animation.fill, Fill::Forwards | Fill::Both);
    if local < 0.0 {
        return backwards.then(|| directed(animation.direction, 0.0, 0.0));
    }
    let active = animation.duration * animation.iterations;
    if animation.duration <= 0.0 || (active.is_finite() && local >= active) {
        if !forwards {
            return None;
        }
        let fraction = animation.iterations.fract();
        let (iteration, p) = if fraction > 0.0 { (animation.iterations.floor(), fraction) } else { ((animation.iterations - 1.0).max(0.0), 1.0) };
        return Some(directed(animation.direction, iteration, p));
    }
    let position = local / animation.duration;
    Some(directed(animation.direction, position.floor(), position.fract()))
}

fn segment<T: Clone>(keyframes: &Keyframes, property: &str, underlying: T, p: f32, parse: impl Fn(&str) -> Option<T>) -> (T, T, f32) {
    let mut points: Vec<(f32, T)> = keyframes
        .frames
        .iter()
        .filter_map(|(offset, declarations)| declarations.get(property).map(|value| (*offset, parse(value).unwrap_or_else(|| underlying.clone()))))
        .collect();
    if points.first().is_none_or(|first| first.0 > 0.0) {
        points.insert(0, (0.0, underlying.clone()));
    }
    if points.last().is_none_or(|last| last.0 < 1.0) {
        points.push((1.0, underlying));
    }
    let index = points.iter().rposition(|point| point.0 <= p).unwrap_or(0).min(points.len() - 2);
    let span = points[index + 1].0 - points[index].0;
    let local = if span > 0.0 { ((p - points[index].0) / span).clamp(0.0, 1.0) } else { 1.0 };
    (points[index].1.clone(), points[index + 1].1.clone(), local)
}

#[cfg(test)]
pub fn sample(animations: &[Animation], elapsed: f32) -> Sampled {
    sample_over(animations, &vec![elapsed; animations.len()], &Sampled::default())
}

pub fn sample_over(animations: &[Animation], elapsed: &[f32], underlying: &Sampled) -> Sampled {
    let mut sampled = underlying.clone();
    for (index, animation) in animations.iter().enumerate() {
        let time = elapsed.get(index).copied().unwrap_or(0.0);
        let (Some(keyframes), Some(p)) = (animation.keyframes.as_deref(), progress(animation, time)) else {
            continue;
        };
        let specifies = |property: &str| keyframes.frames.iter().any(|(_, declarations)| declarations.contains_key(property));
        if specifies("opacity") {
            let (a, b, local) = segment(keyframes, "opacity", underlying.opacity, p, |value| value.trim().parse::<f32>().ok());
            let eased = animation.timing.apply(local);
            sampled.opacity = (a + (b - a) * eased).clamp(0.0, 1.0);
        }
        if specifies("visibility") {
            let (a, b, local) = segment(keyframes, "visibility", underlying.visible, p, |value| Some(value.trim() == "visible"));
            let eased = animation.timing.apply(local);
            sampled.visible = if a == b || eased <= 0.0 {
                a
            } else if eased >= 1.0 {
                b
            } else {
                a || b
            };
        }
        if specifies("color") {
            let base = underlying.color.unwrap_or([1.0, 1.0, 1.0, 1.0]);
            let (a, b, local) = segment(keyframes, "color", base, p, parse_rgba);
            sampled.color = Some(interpolate_color(a, b, animation.timing.apply(local)));
        }
        if specifies("background-color") || specifies("background") {
            let property = if specifies("background-color") { "background-color" } else { "background" };
            let base = underlying.background.unwrap_or([0.0, 0.0, 0.0, 0.0]);
            let (a, b, local) = segment(keyframes, property, base, p, parse_rgba);
            sampled.background = Some(interpolate_color(a, b, animation.timing.apply(local)));
        }
        if specifies("transform") {
            let base = underlying.transform.clone().unwrap_or_default();
            let (a, b, local) = segment(keyframes, "transform", base, p, parse_transform);
            sampled.transform = Some(interpolate_transform(&a, &b, animation.timing.apply(local)));
        }
    }
    sampled
}

pub fn attach(animations: &mut [Animation], keyframes: &BTreeMap<String, Arc<Keyframes>>) {
    for animation in animations {
        animation.keyframes = keyframes.get(&animation.name).cloned();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DZIN: &str = "@keyframes dzin{0%{opacity:0;transform:translateY(-4px)}15%{opacity:1;transform:none}75%{opacity:1}100%{opacity:0;visibility:hidden}}";

    fn dzin() -> Vec<Animation> {
        let mut animations = parse_animation("dzin 4s forwards");
        attach(&mut animations, &parse_keyframes(DZIN));
        animations
    }

    #[test]
    fn keyframes_and_shorthand_parse() {
        let frames = parse_keyframes(&format!(".a{{color:red}}{DZIN}@keyframes p{{from,to{{opacity:.5}}50%{{opacity:1}}}}"));
        assert_eq!(frames["dzin"].frames.iter().map(|frame| frame.0).collect::<Vec<_>>(), vec![0.0, 0.15, 0.75, 1.0]);
        assert_eq!(frames["p"].frames.len(), 3);
        let parsed = parse_animation("coin .25s ease-out,coout .5s ease-in 2s forwards, spin 1s linear infinite");
        assert_eq!(parsed.len(), 3);
        assert_eq!((parsed[0].name.as_str(), parsed[0].duration, parsed[0].timing), ("coin", 0.25, Timing::Bezier(0.0, 0.0, 0.58, 1.0)));
        assert_eq!((parsed[1].delay, parsed[1].fill), (2.0, Fill::Forwards));
        assert!(parsed[2].iterations.is_infinite());
        assert!(parse_animation("none").is_empty());
        assert_eq!(parse_animation("x 1s cubic-bezier(0.1, 0.2, 0.3, 0.4)")[0].timing, Timing::Bezier(0.1, 0.2, 0.3, 0.4));
    }

    #[test]
    fn ease_matches_the_css_curve() {
        assert_eq!(EASE.apply(0.0), 0.0);
        assert!((EASE.apply(1.0) - 1.0).abs() < 1e-5);
        assert!((EASE.apply(0.5) - 0.8024).abs() < 1e-3);
        assert!((Timing::Linear.apply(0.3) - 0.3).abs() < 1e-6);
        assert_eq!(Timing::Steps(4, false).apply(0.3), 0.25);
    }

    #[test]
    fn zone_banner_fades_in_holds_and_ends_hidden() {
        let banner = dzin();
        let start = sample(&banner, 0.0);
        assert!(start.opacity == 0.0 && start.visible);
        assert_eq!(sample(&banner, 0.6).opacity, 1.0);
        let holding = sample(&banner, 2.0);
        assert!(holding.opacity == 1.0 && holding.visible);
        let fading = sample(&banner, 3.5);
        assert!(fading.opacity > 0.0 && fading.opacity < 1.0 && fading.visible);
        assert_eq!(sample(&banner, 4.0).opacity, 0.0);
        assert!(!sample(&banner, 4.0).visible);
        assert!(sample(&banner, 9.0).opacity == 0.0 && !sample(&banner, 9.0).visible);
        let mut no_fill = parse_animation("dzin 4s");
        attach(&mut no_fill, &parse_keyframes(DZIN));
        assert_eq!(sample(&no_fill, 9.0), Sampled::default());
    }

    fn props<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |name| pairs.iter().find(|(key, _)| *key == name).map(|(_, value)| value.to_string())
    }

    #[test]
    fn longhands_build_and_override_animations() {
        let built = apply_animation_longhands(
            Vec::new(),
            props(&[
                ("animation-name", "pulse, glow"),
                ("animation-duration", "2s"),
                ("animation-delay", "0.5s, 1s"),
                ("animation-timing-function", "linear"),
                ("animation-iteration-count", "2, infinite"),
                ("animation-fill-mode", "both"),
                ("animation-play-state", "running, paused"),
            ]),
        );
        assert_eq!(built.len(), 2);
        assert_eq!((built[0].name.as_str(), built[0].duration, built[0].delay, built[0].timing, built[0].iterations, built[0].fill, built[0].paused), ("pulse", 2.0, 0.5, Timing::Linear, 2.0, Fill::Both, false));
        assert_eq!((built[1].name.as_str(), built[1].duration, built[1].delay, built[1].fill, built[1].paused), ("glow", 2.0, 1.0, Fill::Both, true));
        assert!(built[1].iterations.is_infinite());
        let overridden = apply_animation_longhands(parse_animation("dzin 4s forwards"), props(&[("animation-duration", "1s"), ("animation-play-state", "paused")]));
        assert_eq!((overridden[0].duration, overridden[0].fill, overridden[0].paused), (1.0, Fill::Forwards, true));
        assert!(apply_animation_longhands(parse_animation("dzin 4s"), props(&[("animation-name", "none")])).is_empty());
        assert!(parse_animation("x 1s paused")[0].paused);
    }

    #[test]
    fn transitions_parse_from_shorthand_and_longhands() {
        let shorthand = parse_transitions(props(&[("transition", "opacity 1.5s ease-in-out, visibility 1s linear 0.2s")]));
        assert_eq!(shorthand.len(), 2);
        assert_eq!((shorthand[0].property.as_str(), shorthand[0].duration, shorthand[0].timing), ("opacity", 1.5, Timing::Bezier(0.42, 0.0, 0.58, 1.0)));
        assert_eq!((shorthand[1].delay, shorthand[1].timing), (0.2, Timing::Linear));
        let longhands = parse_transitions(props(&[("transition-property", "opacity, visibility"), ("transition-duration", "1s"), ("transition-timing-function", "linear")]));
        assert_eq!(longhands.iter().map(|t| (t.property.as_str(), t.duration)).collect::<Vec<_>>(), vec![("opacity", 1.0), ("visibility", 1.0)]);
        let all = parse_transitions(props(&[("transition", "all .3s")]));
        assert!(transition_for(&all, "opacity").is_some() && transition_for(&all, "visibility").is_some());
        assert!(parse_transitions(props(&[("transition", "none")])).is_empty());
        assert!(parse_transitions(props(&[("transition", "opacity 0s")])).is_empty());
    }

    #[test]
    fn tracks_hold_during_delay_then_ease_to_the_target() {
        let fade = Track { from: 1.0f32, to: 0.25, start: 10.0, duration: 2.0, delay: 0.5, timing: Timing::Linear, reversing_start: 1.0, shortening: 1.0 };
        assert_eq!(fade.value(10.0), 1.0);
        assert_eq!(fade.value(10.5), 1.0);
        assert!((fade.value(11.5) - 0.625).abs() < 1e-6);
        assert_eq!(fade.value(13.0), 0.25);
        let hide = Track { from: true, to: false, start: 0.0, duration: 1.0, delay: 0.0, timing: Timing::Linear, reversing_start: true, shortening: 1.0 };
        assert!(hide.value(0.0) && hide.value(0.99) && !hide.value(1.0));
        let show = Track { from: false, to: true, start: 0.0, duration: 1.0, delay: 0.0, timing: Timing::Linear, reversing_start: false, shortening: 1.0 };
        assert!(!show.value(0.0) && show.value(0.01));
        assert_eq!(Track::settled(0.4f32).value(99.0), 0.4);
    }

    #[test]
    fn animations_sample_over_the_underlying_value() {
        let mut half = parse_animation("p 2s");
        attach(&mut half, &parse_keyframes("@keyframes p{to{opacity:0}}"));
        let under = Sampled { opacity: 0.5, ..Sampled::default() };
        assert!((sample_over(&half, &[0.0], &under).opacity - 0.5).abs() < 1e-6);
        assert_eq!(sample_over(&half, &[5.0], &under).opacity, 0.5);
    }

    fn sweep(direction: &str) -> Vec<Animation> {
        let mut animations = parse_animation(&format!("sweep 1s linear 3 {direction} both"));
        attach(&mut animations, &parse_keyframes("@keyframes sweep{from{opacity:0}to{opacity:1}}"));
        animations
    }

    #[test]
    fn directions_follow_css_directed_progress() {
        let at = |direction: &str, time: f32| sample(&sweep(direction), time).opacity;
        assert!((at("normal", 0.25) - 0.25).abs() < 1e-5 && (at("normal", 1.25) - 0.25).abs() < 1e-5);
        assert!((at("reverse", 0.25) - 0.75).abs() < 1e-5);
        assert!((at("alternate", 0.25) - 0.25).abs() < 1e-5 && (at("alternate", 1.25) - 0.75).abs() < 1e-5 && (at("alternate", 2.25) - 0.25).abs() < 1e-5);
        assert!((at("alternate-reverse", 0.25) - 0.75).abs() < 1e-5 && (at("alternate-reverse", 1.25) - 0.25).abs() < 1e-5);
        assert_eq!((at("normal", 9.0), at("reverse", 9.0), at("alternate", 9.0), at("alternate-reverse", 9.0)), (1.0, 0.0, 1.0, 0.0));
        let mut even = parse_animation("sweep 1s linear 2 alternate forwards");
        attach(&mut even, &parse_keyframes("@keyframes sweep{from{opacity:0}to{opacity:1}}"));
        assert_eq!(sample(&even, 5.0).opacity, 0.0);
        assert_eq!(apply_animation_longhands(parse_animation("x 1s"), |name| (name == "animation-direction").then(|| "alternate-reverse".to_string()))[0].direction, Direction::AlternateReverse);
    }

    #[test]
    fn transforms_parse_and_interpolate() {
        let slide = parse_transform("translateX(30px) scale(1.5) rotate(45deg)").unwrap();
        assert_eq!(slide.len(), 3);
        assert_eq!(parse_transform("none"), Some(Vec::new()));
        assert!(parse_transform("skew(10deg)").is_none());
        let start = parse_transform("translateX(0px) scale(1) rotate(0deg)").unwrap();
        let half = compose(&interpolate_transform(&start, &slide, 0.5));
        assert!((half.tx.px - 15.0).abs() < 1e-5 && (half.sx - 1.25).abs() < 1e-5 && (half.rotation - 22.5f32.to_radians()).abs() < 1e-5);
        let from_none = compose(&interpolate_transform(&[], &parse_transform("translate(20px, 10px)").unwrap(), 0.5));
        assert_eq!((from_none.tx.px, from_none.ty.px), (10.0, 5.0));
        let centred = compose(&interpolate_transform(&parse_transform("translate(-50%, -50%)").unwrap(), &parse_transform("translate(10px, 0px)").unwrap(), 0.5));
        assert_eq!((centred.tx.percent, centred.tx.px), (-25.0, 5.0));
        assert_eq!(centred.tx.resolve(40.0), -5.0);
        assert!((compose(&parse_transform("rotate(0.25turn)").unwrap()).rotation - std::f32::consts::FRAC_PI_2).abs() < 1e-5);
    }

    #[test]
    fn colours_interpolate_in_premultiplied_srgb() {
        let mid = interpolate_color([1.0, 0.0, 0.0, 1.0], [0.0, 1.0, 0.0, 1.0], 0.5);
        assert_eq!(mid, [0.5, 0.5, 0.0, 1.0]);
        let fading = interpolate_color([1.0, 0.0, 0.0, 1.0], [0.0, 0.0, 1.0, 0.0], 0.5);
        assert!((fading[0] - 1.0).abs() < 1e-6 && fading[2].abs() < 1e-6 && (fading[3] - 0.5).abs() < 1e-6);
        let mut tint = parse_animation("tint 2s linear forwards");
        attach(&mut tint, &parse_keyframes("@keyframes tint{from{color:#ff0000;background-color:#0000ff}to{color:#00ff00;background-color:#ffff00}}"));
        let sampled = sample(&tint, 1.0);
        assert_eq!(sampled.color.map(|c| c.map(|v| (v * 1000.0).round() / 1000.0)), Some([0.5, 0.5, 0.0, 1.0]));
        assert_eq!(sampled.background.map(|c| c.map(|v| (v * 1000.0).round() / 1000.0)), Some([0.5, 0.5, 0.5, 1.0]));
    }
}


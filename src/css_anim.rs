use std::collections::BTreeMap;
use std::sync::Arc;

const EASE: Timing = Timing::Bezier(0.25, 0.1, 0.25, 1.0);

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Keyframes {
    frames: Vec<(f32, BTreeMap<String, String>)>,
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

#[derive(Clone, Debug, PartialEq)]
pub struct Animation {
    pub name: String,
    pub duration: f32,
    pub delay: f32,
    pub timing: Timing,
    pub iterations: f32,
    pub fill: Fill,
    pub paused: bool,
    pub keyframes: Option<Arc<Keyframes>>,
}

impl Animation {
    fn named(name: &str) -> Self {
        Self { name: name.to_string(), duration: 0.0, delay: 0.0, timing: EASE, iterations: 1.0, fill: Fill::None, paused: false, keyframes: None }
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
}

impl<T: Copy> Track<T> {
    pub fn settled(value: T) -> Self {
        Self { from: value, to: value, start: 0.0, duration: 0.0, delay: 0.0, timing: Timing::Linear }
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sampled {
    pub opacity: f32,
    pub visible: bool,
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
                } else if matches!(token.as_str(), "none" | "normal" | "reverse" | "alternate" | "alternate-reverse" | "running") {
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

fn progress(animation: &Animation, elapsed: f32) -> Option<f32> {
    let local = elapsed - animation.delay;
    let backwards = matches!(animation.fill, Fill::Backwards | Fill::Both);
    let forwards = matches!(animation.fill, Fill::Forwards | Fill::Both);
    if local < 0.0 {
        return backwards.then_some(0.0);
    }
    let active = animation.duration * animation.iterations;
    if animation.duration <= 0.0 || (active.is_finite() && local >= active) {
        if !forwards {
            return None;
        }
        let fraction = animation.iterations.fract();
        return Some(if fraction > 0.0 { fraction } else { 1.0 });
    }
    Some((local / animation.duration).fract())
}

fn segment<'a>(keyframes: &'a Keyframes, property: &str, underlying: &'a str, p: f32) -> (&'a str, &'a str, f32) {
    let mut points: Vec<(f32, &str)> = keyframes
        .frames
        .iter()
        .filter_map(|(offset, declarations)| declarations.get(property).map(|value| (*offset, value.as_str())))
        .collect();
    if points.first().is_none_or(|first| first.0 > 0.0) {
        points.insert(0, (0.0, underlying));
    }
    if points.last().is_none_or(|last| last.0 < 1.0) {
        points.push((1.0, underlying));
    }
    let index = points.iter().rposition(|point| point.0 <= p).unwrap_or(0).min(points.len() - 2);
    let (from, to) = (points[index], points[index + 1]);
    let span = to.0 - from.0;
    let local = if span > 0.0 { ((p - from.0) / span).clamp(0.0, 1.0) } else { 1.0 };
    (from.1, to.1, local)
}

#[cfg(test)]
pub fn sample(animations: &[Animation], elapsed: f32) -> Sampled {
    sample_over(animations, elapsed, Sampled { opacity: 1.0, visible: true })
}

pub fn sample_over(animations: &[Animation], elapsed: f32, underlying: Sampled) -> Sampled {
    let mut sampled = underlying;
    let base_opacity = underlying.opacity.to_string();
    let base_visibility = if underlying.visible { "visible" } else { "hidden" };
    for animation in animations {
        let (Some(keyframes), Some(p)) = (animation.keyframes.as_deref(), progress(animation, elapsed)) else {
            continue;
        };
        let specifies = |property: &str| keyframes.frames.iter().any(|(_, declarations)| declarations.contains_key(property));
        if specifies("opacity") {
            let (from, to, local) = segment(keyframes, "opacity", &base_opacity, p);
            let eased = animation.timing.apply(local);
            let a = from.parse::<f32>().unwrap_or(1.0);
            let b = to.parse::<f32>().unwrap_or(1.0);
            sampled.opacity = (a + (b - a) * eased).clamp(0.0, 1.0);
        }
        if specifies("visibility") {
            let (from, to, local) = segment(keyframes, "visibility", base_visibility, p);
            let eased = animation.timing.apply(local);
            let value = if from == to {
                from
            } else if eased <= 0.0 {
                from
            } else if eased >= 1.0 {
                to
            } else if from == "visible" || to == "visible" {
                "visible"
            } else if eased < 0.5 {
                from
            } else {
                to
            };
            sampled.visible = value == "visible";
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
        assert_eq!(sample(&banner, 0.0), Sampled { opacity: 0.0, visible: true });
        assert_eq!(sample(&banner, 0.6).opacity, 1.0);
        assert_eq!(sample(&banner, 2.0), Sampled { opacity: 1.0, visible: true });
        let fading = sample(&banner, 3.5);
        assert!(fading.opacity > 0.0 && fading.opacity < 1.0 && fading.visible);
        assert_eq!(sample(&banner, 4.0), Sampled { opacity: 0.0, visible: false });
        assert_eq!(sample(&banner, 9.0), Sampled { opacity: 0.0, visible: false });
        let mut no_fill = parse_animation("dzin 4s");
        attach(&mut no_fill, &parse_keyframes(DZIN));
        assert_eq!(sample(&no_fill, 9.0), Sampled { opacity: 1.0, visible: true });
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
        let fade = Track { from: 1.0f32, to: 0.25, start: 10.0, duration: 2.0, delay: 0.5, timing: Timing::Linear };
        assert_eq!(fade.value(10.0), 1.0);
        assert_eq!(fade.value(10.5), 1.0);
        assert!((fade.value(11.5) - 0.625).abs() < 1e-6);
        assert_eq!(fade.value(13.0), 0.25);
        let hide = Track { from: true, to: false, start: 0.0, duration: 1.0, delay: 0.0, timing: Timing::Linear };
        assert!(hide.value(0.0) && hide.value(0.99) && !hide.value(1.0));
        let show = Track { from: false, to: true, start: 0.0, duration: 1.0, delay: 0.0, timing: Timing::Linear };
        assert!(!show.value(0.0) && show.value(0.01));
        assert_eq!(Track::settled(0.4f32).value(99.0), 0.4);
    }

    #[test]
    fn animations_sample_over_the_underlying_value() {
        let mut half = parse_animation("p 2s");
        attach(&mut half, &parse_keyframes("@keyframes p{to{opacity:0}}"));
        let under = Sampled { opacity: 0.5, visible: true };
        assert!((sample_over(&half, 0.0, under).opacity - 0.5).abs() < 1e-6);
        assert_eq!(sample_over(&half, 5.0, under).opacity, 0.5);
    }
}


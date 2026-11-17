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
    pub keyframes: Option<Arc<Keyframes>>,
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
                } else if matches!(token.as_str(), "none" | "normal" | "reverse" | "alternate" | "alternate-reverse" | "running" | "paused") {
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
                keyframes: None,
            })
        })
        .collect()
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

pub fn sample(animations: &[Animation], elapsed: f32) -> Sampled {
    let mut sampled = Sampled { opacity: 1.0, visible: true };
    for animation in animations {
        let (Some(keyframes), Some(p)) = (animation.keyframes.as_deref(), progress(animation, elapsed)) else {
            continue;
        };
        let specifies = |property: &str| keyframes.frames.iter().any(|(_, declarations)| declarations.contains_key(property));
        if specifies("opacity") {
            let (from, to, local) = segment(keyframes, "opacity", "1", p);
            let eased = animation.timing.apply(local);
            let a = from.parse::<f32>().unwrap_or(1.0);
            let b = to.parse::<f32>().unwrap_or(1.0);
            sampled.opacity = (a + (b - a) * eased).clamp(0.0, 1.0);
        }
        if specifies("visibility") {
            let (from, to, local) = segment(keyframes, "visibility", "visible", p);
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
}

use crate::refusal::Refusal;

pub const MIN_PX: u32 = 64;
pub const MAX_PX: u32 = 1024;
pub const LEAF_DEFAULT_PX: u32 = 128;
pub const BARK_DEFAULT_PX: u32 = 64;
pub const HALVING_LIMIT: u32 = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextureBudget {
    pub leaf_max_px: u32,
    pub bark_max_px: u32,
}

impl Default for TextureBudget {
    fn default() -> Self {
        Self { leaf_max_px: LEAF_DEFAULT_PX, bark_max_px: BARK_DEFAULT_PX }
    }
}

fn refuse(name: &'static str, message: String) -> Refusal {
    Refusal { name, message }
}

fn checked_max_px(max_px: u32, source: &str) -> Result<u32, Refusal> {
    if !max_px.is_power_of_two() {
        return Err(refuse("tree-texture-not-a-power-of-two", format!("{source} = {max_px}")));
    }
    if max_px < MIN_PX {
        return Err(refuse("tree-texture-below-minimum", format!("{source} = {max_px} < {MIN_PX}")));
    }
    if max_px > MAX_PX {
        return Err(refuse("tree-texture-above-maximum", format!("{source} = {max_px} > {MAX_PX}")));
    }
    Ok(max_px)
}

impl TextureBudget {
    pub fn new(leaf_max_px: u32, bark_max_px: u32) -> Result<Self, Refusal> {
        Ok(Self { leaf_max_px: checked_max_px(leaf_max_px, "leaf")?, bark_max_px: checked_max_px(bark_max_px, "bark")? })
    }
}

pub fn halvings_for(width_px: u32, max_px: u32) -> Result<u32, Refusal> {
    if width_px == 0 {
        return Err(refuse("tree-texture-not-a-width", format!("width {width_px}")));
    }
    let target = checked_max_px(max_px, "max_px")?;
    let (mut halvings, mut width) = (0, width_px);
    while width > target && halvings < HALVING_LIMIT {
        width >>= 1;
        halvings += 1;
    }
    Ok(halvings)
}

#[cfg(test)]
pub fn mip_chain_bytes(width_px: u32, height_px: u32) -> u64 {
    let (mut total, mut width, mut height) = (0, width_px, height_px);
    while width >= 1 && height >= 1 {
        total += u64::from(width) * u64::from(height) * 4;
        width >>= 1;
        height >>= 1;
    }
    total
}

pub fn halve_rgba(rgba: &[u8], width: u32, height: u32) -> Vec<u8> {
    let (out_width, out_height) = ((width >> 1).max(1) as usize, (height >> 1).max(1) as usize);
    let at = |x: usize, y: usize, channel: usize| {
        let (x, y) = (x.min(width as usize - 1), y.min(height as usize - 1));
        u32::from(rgba[(y * width as usize + x) * 4 + channel])
    };
    let mut out = Vec::with_capacity(out_width * out_height * 4);
    for y in 0..out_height {
        for x in 0..out_width {
            for channel in 0..4 {
                let sum = at(2 * x, 2 * y, channel) + at(2 * x + 1, 2 * y, channel) + at(2 * x, 2 * y + 1, channel) + at(2 * x + 1, 2 * y + 1, channel);
                out.push(((sum + 2) / 4) as u8);
            }
        }
    }
    out
}

pub fn halved_to_budget(rgba: Vec<u8>, size_px: u32, max_px: u32) -> Result<(Vec<u8>, u32), Refusal> {
    let halvings = halvings_for(size_px, max_px)?;
    Ok((0..halvings).fold((rgba, size_px), |(data, size), _| (halve_rgba(&data, size, size), size >> 1)))
}

pub fn mip_chain(rgba: &[u8], size_px: u32) -> Vec<u8> {
    let mut chain = rgba.to_vec();
    let (mut level, mut size) = (rgba.to_vec(), size_px);
    while size > 1 {
        level = halve_rgba(&level, size, size);
        size >>= 1;
        chain.extend_from_slice(&level);
    }
    chain
}

pub fn mip_levels(size_px: u32) -> u32 {
    size_px.ilog2() + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn halvings_follow_the_reference_budget_arithmetic() {
        assert_eq!(halvings_for(1024, LEAF_DEFAULT_PX).unwrap(), 3);
        assert_eq!(halvings_for(1024, BARK_DEFAULT_PX).unwrap(), 4);
        assert_eq!(halvings_for(1024, 1024).unwrap(), 0);
        assert_eq!(halvings_for(100, 64).unwrap(), 1);
    }

    #[test]
    fn mip_chain_bytes_follow_the_reference() {
        assert_eq!(mip_chain_bytes(128, 128), 87380);
        assert_eq!(mip_chain_bytes(64, 64), 21844);
        assert_eq!(mip_chain_bytes(1024, 1024), 5592404);
        assert_eq!(mip_chain_bytes(3, 5), 68);
    }

    #[test]
    fn the_default_budget_is_the_reference_default_and_bad_values_are_refused() {
        assert_eq!(TextureBudget::default(), TextureBudget { leaf_max_px: 128, bark_max_px: 64 });
        assert_eq!(TextureBudget::new(100, 64).unwrap_err().name, "tree-texture-not-a-power-of-two");
        assert_eq!(TextureBudget::new(32, 64).unwrap_err().name, "tree-texture-below-minimum");
        assert_eq!(TextureBudget::new(128, 2048).unwrap_err().name, "tree-texture-above-maximum");
        assert_eq!(TextureBudget::new(1024, 64).unwrap().leaf_max_px, 1024);
    }

    #[test]
    fn halving_averages_two_by_two_blocks_and_the_chain_ends_at_one_texel() {
        let checker: Vec<u8> = (0..16u8).flat_map(|texel| [texel * 10, 0, 0, 255]).collect();
        let halved = halve_rgba(&checker, 4, 4);
        assert_eq!(halved.len(), 2 * 2 * 4);
        assert_eq!(halved[0], ((0 + 10 + 40 + 50 + 2) / 4) as u8);
        let (data, size) = halved_to_budget(vec![200; 8 * 8 * 4], 8, 64).unwrap();
        assert_eq!((data.len(), size), (8 * 8 * 4, 8));
        assert_eq!(mip_chain(&vec![7; 8 * 8 * 4], 8).len() as u64, mip_chain_bytes(8, 8));
        assert_eq!(mip_levels(2048), 12);
    }
}

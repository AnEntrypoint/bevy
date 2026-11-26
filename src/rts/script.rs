use replay::Command;

pub fn parse(text: &str) -> Result<Vec<(u32, Command)>, String> {
    text.lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .map(|(number, line)| parse_line(line).ok_or_else(|| format!("line {}: {line}", number + 1)))
        .collect()
}

fn parse_line(line: &str) -> Option<(u32, Command)> {
    let fields: Vec<&str> = line.split_whitespace().collect();
    let tick = fields.first()?.parse().ok()?;
    let x = fields.get(3)?.parse().ok()?;
    let y = fields.get(4)?.parse().ok()?;
    let command = match *fields.get(1)? {
        "order" => Command::Order { unit: fields.get(2)?.parse().ok()?, x, y },
        "order_all" => Command::OrderAll { player: fields.get(2)?.parse().ok()?, x, y },
        _ => return None,
    };
    Some((tick, command))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_order_and_order_all() {
        let parsed = parse("0 order 3 100 -200\n16 order_all 1 5 6\n").expect("parses");
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].0, 0);
        assert_eq!(parsed[1].0, 16);
        assert!(matches!(parsed[0].1, Command::Order { unit: 3, x: 100, y: -200 }));
        assert!(matches!(parsed[1].1, Command::OrderAll { player: 1, x: 5, y: 6 }));
    }

    #[test]
    fn names_the_bad_line() {
        let error = parse("0 order 3 100 -200\n4 teleport 1 2 3\n").err().expect("rejects");
        assert_eq!(error, "line 2: 4 teleport 1 2 3");
    }
}

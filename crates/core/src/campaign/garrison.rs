use super::map::GarrisonedUnit;

/// Add units to a garrison, merging with existing entries of the same type.
pub fn add_to_garrison(garrison: &mut Vec<GarrisonedUnit>, unit: GarrisonedUnit) {
    if let Some(existing) = garrison.iter_mut().find(|g| g.unit_type == unit.unit_type) {
        // Weighted average health
        let total_count = existing.count + unit.count;
        if total_count > 0 {
            existing.health_pct = (existing.health_pct * existing.count as f32
                + unit.health_pct * unit.count as f32)
                / total_count as f32;
        }
        existing.count = total_count;
    } else {
        garrison.push(unit);
    }
}

/// Remove units from a garrison. Returns the removed units or None if insufficient.
pub fn remove_from_garrison(
    garrison: &mut Vec<GarrisonedUnit>,
    unit_type: u16,
    count: u32,
) -> Option<GarrisonedUnit> {
    let entry = garrison.iter().find(|g| g.unit_type == unit_type)?;
    if entry.count < count {
        return None;
    }

    let health_pct = entry.health_pct;

    let entry_mut = garrison.iter_mut().find(|g| g.unit_type == unit_type).unwrap();
    entry_mut.count -= count;

    // Clean up empty entries
    garrison.retain(|g| g.count > 0);

    Some(GarrisonedUnit {
        unit_type,
        count,
        health_pct,
    })
}

/// Withdraw all garrison units from a site, returning them.
pub fn withdraw_garrison(garrison: &mut Vec<GarrisonedUnit>) -> Vec<GarrisonedUnit> {
    std::mem::take(garrison)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_to_garrison_new_type() {
        let mut garrison = Vec::new();
        add_to_garrison(&mut garrison, GarrisonedUnit::new(0, 5));
        assert_eq!(garrison.len(), 1);
        assert_eq!(garrison[0].unit_type, 0);
        assert_eq!(garrison[0].count, 5);
    }

    #[test]
    fn test_add_to_garrison_merge() {
        let mut garrison = vec![GarrisonedUnit::new(0, 5)];
        add_to_garrison(&mut garrison, GarrisonedUnit::new(0, 3));
        assert_eq!(garrison.len(), 1, "Should merge same unit type");
        assert_eq!(garrison[0].count, 8);
    }

    #[test]
    fn test_add_to_garrison_health_average() {
        let mut garrison = vec![GarrisonedUnit::with_health(0, 4, 1.0)];
        add_to_garrison(&mut garrison, GarrisonedUnit::with_health(0, 4, 0.5));
        // Average: (1.0 * 4 + 0.5 * 4) / 8 = 0.75
        assert!((garrison[0].health_pct - 0.75).abs() < 0.01);
    }

    #[test]
    fn test_remove_from_garrison() {
        let mut garrison = vec![GarrisonedUnit::new(0, 10)];
        let removed = remove_from_garrison(&mut garrison, 0, 3);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().count, 3);
        assert_eq!(garrison[0].count, 7);
    }

    #[test]
    fn test_remove_from_garrison_all() {
        let mut garrison = vec![GarrisonedUnit::new(0, 5)];
        let removed = remove_from_garrison(&mut garrison, 0, 5);
        assert!(removed.is_some());
        assert!(garrison.is_empty(), "Empty entries should be cleaned up");
    }

    #[test]
    fn test_remove_from_garrison_insufficient() {
        let mut garrison = vec![GarrisonedUnit::new(0, 3)];
        let removed = remove_from_garrison(&mut garrison, 0, 10);
        assert!(removed.is_none(), "Should fail if insufficient units");
        assert_eq!(garrison[0].count, 3, "Garrison should be unchanged");
    }

    #[test]
    fn test_withdraw_garrison() {
        let mut garrison = vec![
            GarrisonedUnit::new(0, 10),
            GarrisonedUnit::new(1, 3),
        ];
        let withdrawn = withdraw_garrison(&mut garrison);
        assert!(garrison.is_empty(), "Garrison should be empty after withdrawal");
        assert_eq!(withdrawn.len(), 2);
        assert_eq!(withdrawn[0].count, 10);
        assert_eq!(withdrawn[1].count, 3);
    }

    #[test]
    fn test_remove_nonexistent_type() {
        let mut garrison = vec![GarrisonedUnit::new(0, 5)];
        let removed = remove_from_garrison(&mut garrison, 1, 1); // type 1 doesn't exist
        assert!(removed.is_none());
    }
}

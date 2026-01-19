//! Weight-based selection utilities

use crate::Endpoint;
use crate::endpoint::WeightType;

const MIN_STATIC_WEIGHT_LIMIT: i32 = 10;
const MAX_STATIC_WEIGHT_LIMIT: i32 = 100;

/// Build a static weight list for weighted round-robin selection
///
/// The returned vector contains endpoint indices, with more occurrences
/// for endpoints with higher weights. This allows simple round-robin
/// selection while respecting weights.
///
/// Returns None if any endpoint doesn't use static weight.
pub fn build_static_weight_list(endpoints: &[Endpoint]) -> Option<Vec<usize>> {
    if endpoints.is_empty() {
        return None;
    }

    // Check all endpoints use static weight
    for ep in endpoints {
        if ep.get_weight_type() != WeightType::StaticWeight {
            return None;
        }
    }

    // Find min and max weights
    let (min_weight, max_weight) = find_weight_range(endpoints)?;

    // Calculate range
    let mut max_range = max_weight / min_weight;
    if max_range < MIN_STATIC_WEIGHT_LIMIT {
        max_range = MIN_STATIC_WEIGHT_LIMIT;
    }
    if max_range > MAX_STATIC_WEIGHT_LIMIT {
        max_range = MAX_STATIC_WEIGHT_LIMIT;
    }

    // Normalize weights
    let normalized: Vec<i32> = endpoints
        .iter()
        .map(|ep| {
            let weight = ep.weight as i32;
            let normalized = (weight * max_range) / max_weight;
            normalized.max(1)
        })
        .collect();

    // Build weighted list
    build_weighted_list(&normalized)
}

/// Find min and max weights from endpoints
fn find_weight_range(endpoints: &[Endpoint]) -> Option<(i32, i32)> {
    if endpoints.is_empty() {
        return None;
    }

    let mut min_weight = i32::MAX;
    let mut max_weight = i32::MIN;

    for ep in endpoints {
        let weight = (ep.weight as i32).max(1); // Ensure positive weight
        min_weight = min_weight.min(weight);
        max_weight = max_weight.max(weight);
    }

    Some((min_weight, max_weight))
}

/// Build weighted selection list from normalized weights
///
/// Uses smooth weighted round-robin to distribute selections evenly.
fn build_weighted_list(weights: &[i32]) -> Option<Vec<usize>> {
    if weights.is_empty() {
        return None;
    }

    let total: i32 = weights.iter().sum();
    if total == 0 {
        return None;
    }

    // Calculate GCD to reduce list size
    let gcd = weights.iter().fold(0, |acc, &w| gcd(acc, w));
    let reduced: Vec<i32> = weights.iter().map(|&w| w / gcd).collect();
    let total_reduced: i32 = reduced.iter().sum();

    // Build the list with smooth distribution
    let mut list = Vec::with_capacity(total_reduced as usize);
    let mut current_weights: Vec<i32> = vec![0; reduced.len()];

    for _ in 0..total_reduced {
        // Add weights
        for (i, &w) in reduced.iter().enumerate() {
            current_weights[i] += w;
        }

        // Find max and select
        let (max_idx, _) = current_weights
            .iter()
            .enumerate()
            .max_by_key(|(_, &w)| w)
            .unwrap();

        list.push(max_idx);

        // Subtract total from selected
        current_weights[max_idx] -= total_reduced;
    }

    Some(list)
}

/// Calculate greatest common divisor
fn gcd(a: i32, b: i32) -> i32 {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_weighted_endpoint(port: u16, weight: u32) -> Endpoint {
        Endpoint {
            host: "127.0.0.1".to_string(),
            port,
            weight,
            weight_type: WeightType::StaticWeight.as_i16(),
            ..Default::default()
        }
    }

    #[test]
    fn test_gcd() {
        assert_eq!(gcd(12, 8), 4);
        assert_eq!(gcd(100, 25), 25);
        assert_eq!(gcd(7, 3), 1);
    }

    #[test]
    fn test_build_weight_list_empty() {
        assert!(build_static_weight_list(&[]).is_none());
    }

    #[test]
    fn test_build_weight_list_non_static() {
        let endpoints = vec![Endpoint::tcp("127.0.0.1", 10000)];
        assert!(build_static_weight_list(&endpoints).is_none());
    }

    #[test]
    fn test_build_weight_list_equal_weights() {
        let endpoints = vec![
            make_weighted_endpoint(10000, 100),
            make_weighted_endpoint(10001, 100),
            make_weighted_endpoint(10002, 100),
        ];

        let list = build_static_weight_list(&endpoints).unwrap();

        // Count occurrences - should be roughly equal
        let counts: std::collections::HashMap<_, _> =
            list.iter().fold(std::collections::HashMap::new(), |mut acc, &idx| {
                *acc.entry(idx).or_insert(0) += 1;
                acc
            });

        for count in counts.values() {
            assert!(*count > 0);
        }
    }

    #[test]
    fn test_build_weight_list_different_weights() {
        let endpoints = vec![
            make_weighted_endpoint(10000, 100),
            make_weighted_endpoint(10001, 200),
        ];

        let list = build_static_weight_list(&endpoints).unwrap();

        // Count occurrences
        let count_0 = list.iter().filter(|&&idx| idx == 0).count();
        let count_1 = list.iter().filter(|&&idx| idx == 1).count();

        // Endpoint 1 should have roughly 2x the selections
        println!("Count 0: {}, Count 1: {}", count_0, count_1);
        assert!(count_1 > count_0);
    }

    #[test]
    fn test_weight_distribution() {
        let endpoints = vec![
            make_weighted_endpoint(10000, 10),
            make_weighted_endpoint(10001, 30),
            make_weighted_endpoint(10002, 60),
        ];

        let list = build_static_weight_list(&endpoints).unwrap();

        let counts: Vec<usize> = (0..3)
            .map(|i| list.iter().filter(|&&idx| idx == i).count())
            .collect();

        println!("Distribution: {:?}", counts);

        // Weights are 10:30:60 = 1:3:6
        // So distribution should roughly follow this ratio
        assert!(counts[2] > counts[1]);
        assert!(counts[1] > counts[0]);
    }
}

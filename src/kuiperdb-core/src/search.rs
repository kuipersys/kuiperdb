use crate::DistanceMetric;

pub(crate) fn distance(metric: DistanceMetric, left: &[f32], right: &[f32]) -> f32 {
    match metric {
        DistanceMetric::Cosine => cosine_distance(left, right),
        DistanceMetric::Euclidean => left
            .iter()
            .zip(right)
            .map(|(a, b)| (a - b).powi(2))
            .sum::<f32>()
            .sqrt(),
        // Negating the dot product makes "lower is nearer" consistent across metrics.
        DistanceMetric::DotProduct => -left.iter().zip(right).map(|(a, b)| a * b).sum::<f32>(),
    }
}

fn cosine_distance(left: &[f32], right: &[f32]) -> f32 {
    let dot = left.iter().zip(right).map(|(a, b)| a * b).sum::<f32>();
    let left_norm = left.iter().map(|value| value * value).sum::<f32>().sqrt();
    let right_norm = right.iter().map(|value| value * value).sum::<f32>().sqrt();
    if left_norm == 0.0 || right_norm == 0.0 {
        1.0
    } else {
        1.0 - dot / (left_norm * right_norm)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metrics_have_lower_distance_for_nearer_vectors() {
        let query = [1.0, 0.0];
        for metric in [
            DistanceMetric::Cosine,
            DistanceMetric::Euclidean,
            DistanceMetric::DotProduct,
        ] {
            assert!(distance(metric, &query, &[1.0, 0.0]) < distance(metric, &query, &[0.0, 1.0]));
        }
    }
}

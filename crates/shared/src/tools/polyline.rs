/*  Copyright 2022-23, Juspay India Pvt Ltd
    This program is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License
    as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version. This program
    is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY
    or FITNESS FOR A PARTICULAR PURPOSE. See the GNU Affero General Public License for more details. You should have received a copy of
    the GNU Affero General Public License along with this program. If not, see <https://www.gnu.org/licenses/>.
*/

//! Polyline spatial index for O(log N) nearest-point and along-polyline distance lookups.
//!
//! Buckets polyline points by precision-7 geohash (~153m cells). `find_nearest_point`
//! checks the center cell plus its 8 neighbors before falling back to a linear scan.
//! `distance_between` uses a precomputed cumulative-distance array.
//!
//! Extracted from eta-compute so multiple Rust services (gps-processor, future workers)
//! can share one implementation.

use std::collections::HashMap;

const EARTH_RADIUS_M: f64 = 6_371_000.0;
const GEOHASH_PRECISION: usize = 7;

/// Haversine distance between two (lat, lng) points in meters.
pub fn haversine_meters(from: (f64, f64), to: (f64, f64)) -> f64 {
    let (lat1, lng1) = from;
    let (lat2, lng2) = to;
    let lat1_r = lat1.to_radians();
    let lat2_r = lat2.to_radians();
    let dlat = (lat2 - lat1).to_radians();
    let dlng = (lng2 - lng1).to_radians();
    let a = (dlat / 2.0).sin().powi(2) + lat1_r.cos() * lat2_r.cos() * (dlng / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
    EARTH_RADIUS_M * c
}

/// Spatial index over a polyline.
pub struct PolylineIndex {
    cumulative_distances: Vec<f64>,
    geohash_buckets: HashMap<String, Vec<usize>>,
    points: Vec<(f64, f64)>,
}

impl PolylineIndex {
    pub fn build(points: &[(f64, f64)]) -> Self {
        let points: Vec<(f64, f64)> = points.to_vec();

        let mut cumulative_distances = Vec::with_capacity(points.len());
        if !points.is_empty() {
            cumulative_distances.push(0.0);
            for i in 1..points.len() {
                let prev_cum = cumulative_distances[i - 1];
                cumulative_distances.push(prev_cum + haversine_meters(points[i - 1], points[i]));
            }
        }

        let mut geohash_buckets: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, &(lat, lng)) in points.iter().enumerate() {
            if let Ok(gh) = geohash::encode(geohash::Coord { x: lng, y: lat }, GEOHASH_PRECISION) {
                geohash_buckets.entry(gh).or_default().push(i);
            }
        }

        Self {
            cumulative_distances,
            geohash_buckets,
            points,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    pub fn len(&self) -> usize {
        self.points.len()
    }

    /// Find the index of the polyline point nearest to (lat, lng) and the distance in meters.
    /// Returns None if the index is empty.
    pub fn find_nearest_point(&self, lat: f64, lng: f64) -> Option<(usize, f64)> {
        if self.points.is_empty() {
            return None;
        }

        let gh = geohash::encode(geohash::Coord { x: lng, y: lat }, GEOHASH_PRECISION).ok()?;

        let mut candidates: Vec<&Vec<usize>> = Vec::new();
        if let Some(bucket) = self.geohash_buckets.get(&gh) {
            candidates.push(bucket);
        }
        if let Ok(n) = geohash::neighbors(&gh) {
            for neighbor_gh in [&n.n, &n.ne, &n.e, &n.se, &n.s, &n.sw, &n.w, &n.nw] {
                if let Some(bucket) = self.geohash_buckets.get(neighbor_gh) {
                    candidates.push(bucket);
                }
            }
        }

        if candidates.is_empty() {
            return self.find_nearest_point_linear(lat, lng);
        }

        let mut min_dist = f64::MAX;
        let mut best_idx = 0;
        for bucket in candidates {
            for &idx in bucket {
                let d = haversine_meters((lat, lng), self.points[idx]);
                if d < min_dist {
                    min_dist = d;
                    best_idx = idx;
                }
            }
        }

        Some((best_idx, min_dist))
    }

    fn find_nearest_point_linear(&self, lat: f64, lng: f64) -> Option<(usize, f64)> {
        if self.points.is_empty() {
            return None;
        }
        let mut min_dist = f64::MAX;
        let mut best_idx = 0;
        for (i, &p) in self.points.iter().enumerate() {
            let d = haversine_meters((lat, lng), p);
            if d < min_dist {
                min_dist = d;
                best_idx = i;
            }
        }
        Some((best_idx, min_dist))
    }

    /// Along-polyline distance between the points nearest to `from` and `to`, in meters.
    /// Uses the precomputed cumulative-distance array. Returns None if either lookup fails.
    pub fn distance_between(&self, from: (f64, f64), to: (f64, f64)) -> Option<f64> {
        let (from_idx, _) = self.find_nearest_point(from.0, from.1)?;
        let (to_idx, _) = self.find_nearest_point(to.0, to.1)?;
        Some((self.cumulative_distances[to_idx] - self.cumulative_distances[from_idx]).abs())
    }
}

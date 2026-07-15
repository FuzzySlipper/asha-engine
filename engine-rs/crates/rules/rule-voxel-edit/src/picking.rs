//! Authority-safe picking / edit-anchor validation (voxel-capability-10).
//!
//! The renderer/UI builds a screen→world ray and may do its own visual picking
//! for responsiveness, but a UI-claimed hit is **only a hint**. Before any edit is
//! accepted, Rust revalidates the hint against the authoritative collision
//! projection through the *same shared query service* (no parallel DDA/raycast),
//! and turns a validated anchor into a canonical [`VoxelCommand`]. The UI never
//! mutates authoritative voxel state.
//!
//! ```text
//! TS screen ray + claimed hit ─▶ validate_pick(projection, hint) ─▶ ValidatedAnchor
//!   ▶ place_command / remove_command ─▶ VoxelCommand ─▶ rule_voxel_edit::validate ─▶ apply
//!                                  └─▶ PickRejection (stale / mismatched / no hit)
//! ```

use core_commands::VoxelCommand;
use core_scene::transform::{SceneTransform, TransformInvalid};
use core_space::{Face, GridId, VoxelCoord};
use core_voxel::VoxelValue;
use svc_collision::{CollisionProjection, Ray, VoxelHit};

/// An untrusted pick proposed by the renderer/UI: the ray it cast plus the voxel
/// and face it believes were hit. Revalidated by [`validate_pick`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RendererPickHint {
    pub ray: Ray,
    pub claimed_voxel: VoxelCoord,
    pub claimed_face: Face,
}

/// A pick that Rust has confirmed against authoritative state. Carries the
/// authoritative [`VoxelHit`] and the resolved edit anchors.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ValidatedAnchor {
    /// The authoritative hit (its `voxel` is the cell a *remove* targets).
    pub hit: VoxelHit,
    /// The empty neighbour across the struck face — the cell a *place* targets.
    pub place_anchor: VoxelCoord,
}

/// Why a renderer pick hint was refused by authoritative revalidation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PickRejection {
    /// The authoritative raycast hit nothing within range — the UI pick was stale
    /// or aimed at empty space.
    NoHit,
    /// The authoritative hit disagrees with the claimed hit (stale projection or a
    /// renderer/authority mismatch). Carries both so a diagnostic overlay can show
    /// the discrepancy.
    HitMismatch {
        authoritative: VoxelHit,
        claimed_voxel: VoxelCoord,
        claimed_face: Face,
    },
}

/// Why a world-space instance ray could not be transformed into an authoritative
/// asset-local query. This is classified before collision invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstancePickInvalid {
    InvalidTransform(TransformInvalid),
    InvalidRay,
}

/// Authority result for an instanced voxel pick. The cell, face, and edit anchor
/// remain asset-local; the point and distance are returned in caller world space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InstanceValidatedAnchor {
    pub local: ValidatedAnchor,
    pub world_point: [f64; 3],
    pub world_distance: f64,
}

/// Revalidate a renderer pick hint against the authoritative collision projection.
/// Accepts only when the authoritative nearest hit matches the claimed voxel+face.
pub fn validate_pick(
    projection: &CollisionProjection,
    hint: &RendererPickHint,
    max_distance: f64,
) -> Result<ValidatedAnchor, PickRejection> {
    match projection.raycast(hint.ray, max_distance) {
        None => Err(PickRejection::NoHit),
        Some(hit) if hit.voxel == hint.claimed_voxel && hit.face == hint.claimed_face => {
            Ok(ValidatedAnchor {
                hit,
                place_anchor: hit.voxel.neighbor(hit.face),
            })
        }
        Some(hit) => Err(PickRejection::HitMismatch {
            authoritative: hit,
            claimed_voxel: hint.claimed_voxel,
            claimed_face: hint.claimed_face,
        }),
    }
}

/// Revalidate a renderer hint against an asset-local collision projection after
/// applying the inverse of the retained instance transform in Rust.
///
/// The world ray is normalized before inverse TRS. Non-uniform scale changes the
/// local ray's length and therefore its local maximum distance; the conversion
/// below preserves the caller's world-distance bound. Renderer-provided cells are
/// only hints and are compared with the independently recomputed local hit.
pub fn validate_instance_pick(
    projection: &CollisionProjection,
    transform: SceneTransform,
    world_ray: Ray,
    max_world_distance: f64,
    claimed_local_voxel: VoxelCoord,
    claimed_local_face: Face,
) -> Result<InstanceValidatedAnchor, InstancePickInvalidOrRejected> {
    let local = inverse_transform_ray(transform, world_ray, max_world_distance)
        .map_err(InstancePickInvalidOrRejected::Invalid)?;
    let hint = RendererPickHint {
        ray: local.ray,
        claimed_voxel: claimed_local_voxel,
        claimed_face: claimed_local_face,
    };
    let anchor = validate_pick(projection, &hint, local.max_distance)
        .map_err(InstancePickInvalidOrRejected::Rejected)?;
    let world_point = transform_point(transform, anchor.hit.point);
    Ok(InstanceValidatedAnchor {
        local: anchor,
        world_point: [world_point.x, world_point.y, world_point.z],
        world_distance: anchor.hit.distance / local.distance_scale,
    })
}

/// Classified union for transformed pick validation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InstancePickInvalidOrRejected {
    Invalid(InstancePickInvalid),
    Rejected(PickRejection),
}

#[derive(Debug, Clone, Copy)]
struct LocalRay {
    ray: Ray,
    max_distance: f64,
    distance_scale: f64,
}

fn inverse_transform_ray(
    transform: SceneTransform,
    world_ray: Ray,
    max_world_distance: f64,
) -> Result<LocalRay, InstancePickInvalid> {
    transform
        .validate()
        .map_err(InstancePickInvalid::InvalidTransform)?;
    let world_length = world_ray.dir.length();
    if !world_length.is_finite()
        || world_length <= 0.0
        || !max_world_distance.is_finite()
        || max_world_distance <= 0.0
    {
        return Err(InstancePickInvalid::InvalidRay);
    }
    let world_direction = [
        world_ray.dir.x / world_length,
        world_ray.dir.y / world_length,
        world_ray.dir.z / world_length,
    ];
    let translated_origin = [
        world_ray.origin.x - transform.translation.x as f64,
        world_ray.origin.y - transform.translation.y as f64,
        world_ray.origin.z - transform.translation.z as f64,
    ];
    let local_origin_rotated = inverse_rotate(transform, translated_origin);
    let local_direction_rotated = inverse_rotate(transform, world_direction);
    let scale = [
        transform.scale.x as f64,
        transform.scale.y as f64,
        transform.scale.z as f64,
    ];
    let local_origin = [
        local_origin_rotated[0] / scale[0],
        local_origin_rotated[1] / scale[1],
        local_origin_rotated[2] / scale[2],
    ];
    let local_direction = [
        local_direction_rotated[0] / scale[0],
        local_direction_rotated[1] / scale[1],
        local_direction_rotated[2] / scale[2],
    ];
    let distance_scale = vector_length(local_direction);
    if !distance_scale.is_finite() || distance_scale <= 0.0 {
        return Err(InstancePickInvalid::InvalidRay);
    }
    Ok(LocalRay {
        ray: Ray::new(
            core_space::WorldPos::new(local_origin[0], local_origin[1], local_origin[2]),
            core_space::WorldVec::new(local_direction[0], local_direction[1], local_direction[2]),
        ),
        max_distance: max_world_distance * distance_scale,
        distance_scale,
    })
}

fn inverse_rotate(transform: SceneTransform, vector: [f64; 3]) -> [f64; 3] {
    let q = normalized_quaternion(transform);
    rotate_vector([-q[0], -q[1], -q[2], q[3]], vector)
}

fn transform_point(transform: SceneTransform, point: core_space::WorldPos) -> core_space::WorldPos {
    let scaled = [
        point.x * transform.scale.x as f64,
        point.y * transform.scale.y as f64,
        point.z * transform.scale.z as f64,
    ];
    let rotated = rotate_vector(normalized_quaternion(transform), scaled);
    core_space::WorldPos::new(
        rotated[0] + transform.translation.x as f64,
        rotated[1] + transform.translation.y as f64,
        rotated[2] + transform.translation.z as f64,
    )
}

fn normalized_quaternion(transform: SceneTransform) -> [f64; 4] {
    let q = [
        transform.rotation.x as f64,
        transform.rotation.y as f64,
        transform.rotation.z as f64,
        transform.rotation.w as f64,
    ];
    let inverse_norm = 1.0 / (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt();
    [
        q[0] * inverse_norm,
        q[1] * inverse_norm,
        q[2] * inverse_norm,
        q[3] * inverse_norm,
    ]
}

fn rotate_vector(q: [f64; 4], v: [f64; 3]) -> [f64; 3] {
    let u = [q[0], q[1], q[2]];
    let uv = cross(u, v);
    let uuv = cross(u, uv);
    [
        v[0] + 2.0 * (q[3] * uv[0] + uuv[0]),
        v[1] + 2.0 * (q[3] * uv[1] + uuv[1]),
        v[2] + 2.0 * (q[3] * uv[2] + uuv[2]),
    ]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn vector_length(v: [f64; 3]) -> f64 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

/// Build a canonical *place* command from a validated anchor (sets the empty cell
/// across the struck face). Still subject to [`crate::validate`] (material/resident).
pub fn place_command(grid: GridId, anchor: &ValidatedAnchor, value: VoxelValue) -> VoxelCommand {
    VoxelCommand::SetVoxel {
        grid,
        coord: anchor.place_anchor,
        value,
    }
}

/// Build a canonical *remove* command from a validated anchor (clears the hit cell).
pub fn remove_command(grid: GridId, anchor: &ValidatedAnchor) -> VoxelCommand {
    VoxelCommand::SetVoxel {
        grid,
        coord: anchor.hit.voxel,
        value: VoxelValue::EMPTY,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{apply_all, validate};
    use core_math::Vec3;
    use core_scene::transform::Quat;
    use core_space::{ChunkCoord, ChunkDims, LocalVoxelCoord, VoxelGridSpec, WorldPos, WorldVec};
    use core_voxel::{MaterialCatalog, VoxelMaterialId};
    use svc_spatial::VoxelWorld;
    use svc_volume::VoxelChunk;

    fn spec() -> VoxelGridSpec {
        VoxelGridSpec::new(GridId::new(0), 1.0, ChunkDims::cubic(8).unwrap()).unwrap()
    }

    fn materials() -> MaterialCatalog {
        MaterialCatalog::new([VoxelMaterialId::new(1)])
    }

    fn world_with_solid(local: LocalVoxelCoord) -> VoxelWorld {
        let mut w = VoxelWorld::new(spec());
        let mut chunk = VoxelChunk::from_spec(&spec());
        chunk.set(local, VoxelValue::solid_raw(1)).unwrap();
        w.insert(ChunkCoord::new(0, 0, 0), chunk);
        w.drain_dirty();
        w
    }

    fn ray_plus_x() -> Ray {
        Ray::new(WorldPos::new(0.0, 0.5, 0.5), WorldVec::new(1.0, 0.0, 0.0))
    }

    #[test]
    fn matching_renderer_hint_is_accepted_and_yields_place_remove_commands() {
        let world = world_with_solid(LocalVoxelCoord::new(5, 0, 0)); // world voxel (5,0,0)
        let proj = CollisionProjection::build(&world);
        // The renderer claims exactly what the authoritative raycast finds.
        let truth = proj.raycast(ray_plus_x(), 100.0).unwrap();
        let hint = RendererPickHint {
            ray: ray_plus_x(),
            claimed_voxel: truth.voxel,
            claimed_face: truth.face,
        };
        let anchor = validate_pick(&proj, &hint, 100.0).expect("matching hint accepted");
        assert_eq!(anchor.hit.voxel, VoxelCoord::new(5, 0, 0));
        assert_eq!(anchor.place_anchor, VoxelCoord::new(4, 0, 0)); // across the -X face

        // Place command targets the empty neighbour and survives authority validation.
        let mut world = world;
        let place = place_command(GridId::new(0), &anchor, VoxelValue::solid_raw(1));
        let events = validate(&place, &world, &materials()).unwrap();
        apply_all(&mut world, &events).unwrap();
        assert_eq!(
            world
                .get(ChunkCoord::new(0, 0, 0))
                .unwrap()
                .get(LocalVoxelCoord::new(4, 0, 0)),
            Some(VoxelValue::solid_raw(1)),
        );

        // Remove command clears the hit cell.
        let remove = remove_command(GridId::new(0), &anchor);
        let revents = validate(&remove, &world, &materials()).unwrap();
        apply_all(&mut world, &revents).unwrap();
        assert_eq!(
            world
                .get(ChunkCoord::new(0, 0, 0))
                .unwrap()
                .get(LocalVoxelCoord::new(5, 0, 0)),
            Some(VoxelValue::EMPTY),
        );
    }

    #[test]
    fn stale_renderer_hint_is_rejected_by_authority() {
        let world = world_with_solid(LocalVoxelCoord::new(5, 0, 0));
        let proj = CollisionProjection::build(&world);
        // Renderer claims a wrong voxel/face (e.g. its projection was stale).
        let hint = RendererPickHint {
            ray: ray_plus_x(),
            claimed_voxel: VoxelCoord::new(2, 0, 0), // not what authority sees
            claimed_face: Face::NegX,
        };
        match validate_pick(&proj, &hint, 100.0) {
            Err(PickRejection::HitMismatch {
                authoritative,
                claimed_voxel,
                ..
            }) => {
                assert_eq!(authoritative.voxel, VoxelCoord::new(5, 0, 0));
                assert_eq!(claimed_voxel, VoxelCoord::new(2, 0, 0));
            }
            other => panic!("expected HitMismatch, got {other:?}"),
        }
    }

    #[test]
    fn hint_into_empty_space_is_rejected_no_hit() {
        let world = world_with_solid(LocalVoxelCoord::new(5, 0, 0));
        let proj = CollisionProjection::build(&world);
        // Ray that never enters the solid cell.
        let hint = RendererPickHint {
            ray: Ray::new(WorldPos::new(0.0, 3.5, 0.5), WorldVec::new(1.0, 0.0, 0.0)),
            claimed_voxel: VoxelCoord::new(5, 0, 0),
            claimed_face: Face::NegX,
        };
        assert_eq!(
            validate_pick(&proj, &hint, 100.0),
            Err(PickRejection::NoHit)
        );
    }

    #[test]
    fn validation_does_not_mutate_authority_or_projection() {
        let world = world_with_solid(LocalVoxelCoord::new(5, 0, 0));
        let proj = CollisionProjection::build(&world);
        let before_version = proj.version();
        let before_hash = world.get(ChunkCoord::new(0, 0, 0)).unwrap().content_hash();
        let truth = proj.raycast(ray_plus_x(), 100.0).unwrap();
        let hint = RendererPickHint {
            ray: ray_plus_x(),
            claimed_voxel: truth.voxel,
            claimed_face: truth.face,
        };
        let _ = validate_pick(&proj, &hint, 100.0).unwrap();
        // Pick validation is read-only over authority + projection.
        assert_eq!(proj.version(), before_version);
        assert_eq!(
            world.get(ChunkCoord::new(0, 0, 0)).unwrap().content_hash(),
            before_hash
        );
    }

    #[test]
    fn translated_rotated_non_uniform_instance_pick_returns_local_edit_anchor() {
        let world = world_with_solid(LocalVoxelCoord::new(5, 0, 0));
        let projection = CollisionProjection::build(&world);
        let half_sqrt = std::f32::consts::FRAC_1_SQRT_2;
        let transform = SceneTransform::new(
            Vec3::new(10.0, 20.0, 30.0),
            Quat::new(0.0, 0.0, half_sqrt, half_sqrt),
            Vec3::new(2.0, 3.0, 0.5),
        );
        // Local +X maps to world +Y under the 90-degree Z rotation. Local
        // (0,.5,.5) maps to world (8.5,20,30.25) after non-uniform scale.
        let ray = Ray::new(
            WorldPos::new(8.5, 20.0, 30.25),
            WorldVec::new(0.0, 1.0, 0.0),
        );
        let picked = validate_instance_pick(
            &projection,
            transform,
            ray,
            20.0,
            VoxelCoord::new(5, 0, 0),
            Face::NegX,
        )
        .expect("transformed authority pick");

        assert_eq!(picked.local.hit.voxel, VoxelCoord::new(5, 0, 0));
        assert_eq!(picked.local.place_anchor, VoxelCoord::new(4, 0, 0));
        assert!((picked.world_point[0] - 8.5).abs() < 1e-5);
        assert!((picked.world_point[1] - 30.0).abs() < 1e-5);
        assert!((picked.world_point[2] - 30.25).abs() < 1e-5);
        assert!((picked.world_distance - 10.0).abs() < 1e-5);
    }

    #[test]
    fn transformed_pick_still_rejects_renderer_local_cell_hint() {
        let world = world_with_solid(LocalVoxelCoord::new(5, 0, 0));
        let projection = CollisionProjection::build(&world);
        let transform = SceneTransform::new(
            Vec3::new(10.0, 0.0, 0.0),
            Quat::IDENTITY,
            Vec3::new(2.0, 1.0, 1.0),
        );
        let result = validate_instance_pick(
            &projection,
            transform,
            Ray::new(WorldPos::new(10.0, 0.5, 0.5), WorldVec::new(1.0, 0.0, 0.0)),
            20.0,
            VoxelCoord::new(4, 0, 0),
            Face::NegX,
        );
        assert!(matches!(
            result,
            Err(InstancePickInvalidOrRejected::Rejected(
                PickRejection::HitMismatch { .. }
            ))
        ));
    }
}

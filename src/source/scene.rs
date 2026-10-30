use std::{
    ffi::{CString, c_void},
    panic::{AssertUnwindSafe, catch_unwind, resume_unwind},
};

use crate::{source::SourceRef, wrapper::PtrWrapper};
use obs_sys_rs::{
    obs_bounds_type, obs_bounds_type_OBS_BOUNDS_NONE, obs_enum_scenes, obs_scene_add,
    obs_scene_enum_items, obs_scene_from_source, obs_scene_get_ref, obs_scene_get_source,
    obs_scene_release, obs_scene_t, obs_sceneitem_addref, obs_sceneitem_crop,
    obs_sceneitem_defer_update_begin, obs_sceneitem_defer_update_end, obs_sceneitem_get_crop,
    obs_sceneitem_get_id, obs_sceneitem_get_info2, obs_sceneitem_get_source,
    obs_sceneitem_group_get_scene, obs_sceneitem_is_group, obs_sceneitem_locked,
    obs_sceneitem_release, obs_sceneitem_set_crop, obs_sceneitem_set_info2,
    obs_sceneitem_set_scale, obs_sceneitem_t, obs_sceneitem_visible, obs_source_t,
    obs_transform_info, vec2,
};

use super::Result;

/// A reference-counted handle to an OBS scene (`obs_scene_t`).
///
/// A scene is a layered composition of sources. `SceneRef` exposes the
/// scene's identity, conversion to the underlying [`SourceRef`], and a
/// helper for adding a source as a new scene item.
pub struct SceneRef {
    inner: *mut obs_scene_t,
}

impl std::fmt::Debug for SceneRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("SceneRef")
            .field(&self.name())
            .field(&self.inner)
            .finish()
    }
}

impl_ptr_wrapper!(@ptr: inner, SceneRef, obs_scene_t, obs_scene_get_ref, obs_scene_release);

/// Shared state for the enumeration trampolines. A panic in the Rust
/// callback stops enumeration and is resumed once libobs has returned and
/// released its locks, so it never unwinds through C.
struct Enumeration<F> {
    callback: F,
    panic: Option<Box<dyn std::any::Any + Send>>,
}

impl<F> Enumeration<F> {
    fn new(callback: F) -> Self {
        Self {
            callback,
            panic: None,
        }
    }

    fn call(&mut self, visit: impl FnOnce(&mut F) -> bool) -> bool {
        match catch_unwind(AssertUnwindSafe(|| visit(&mut self.callback))) {
            Ok(keep_going) => keep_going,
            Err(payload) => {
                self.panic = Some(payload);
                false
            }
        }
    }

    fn finish(self) {
        if let Some(payload) = self.panic {
            resume_unwind(payload);
        }
    }
}

unsafe extern "C" fn enum_items_proc<F: FnMut(&SceneItemRef) -> bool>(
    _scene: *mut obs_scene_t,
    item: *mut obs_sceneitem_t,
    param: *mut c_void,
) -> bool {
    let state = unsafe { &mut *param.cast::<Enumeration<F>>() };
    // libobs holds its own item reference for the duration of the call.
    let Some(item) = SceneItemRef::from_raw(item) else {
        return true;
    };
    state.call(|callback| callback(&item))
}

unsafe extern "C" fn enum_scenes_proc<F: FnMut(&SourceRef) -> bool>(
    param: *mut c_void,
    source: *mut obs_source_t,
) -> bool {
    let state = unsafe { &mut *param.cast::<Enumeration<F>>() };
    let Some(source) = SourceRef::from_raw(source) else {
        return true;
    };
    state.call(|callback| callback(&source))
}

impl SceneRef {
    /// Returns the scene backing `source`, or `None` if `source` is not a
    /// scene. Groups are not scenes here; use [`SceneItemRef::group_scene`].
    pub fn from_source(source: &SourceRef) -> Option<Self> {
        // obs_scene_from_source does not take a reference; from_raw does.
        Self::from_raw(unsafe { obs_scene_from_source(source.as_ptr()) })
    }

    /// Invokes `callback` for each scene source in the main canvas,
    /// including groups, until it returns `false` (`obs_enum_scenes`).
    ///
    /// libobs holds its source-list lock throughout, so `callback` must not
    /// create or remove sources. A panic stops enumeration and is resumed
    /// after the lock is released.
    pub fn enum_scenes<F: FnMut(&SourceRef) -> bool>(callback: F) {
        let mut state = Enumeration::new(callback);
        unsafe {
            obs_enum_scenes(
                Some(enum_scenes_proc::<F>),
                (&mut state as *mut Enumeration<F>).cast(),
            );
        }
        state.finish();
    }

    /// Returns the user-visible name of the scene.
    pub fn name(&self) -> Result<CString> {
        self.as_source().name()
    }

    /// Returns the underlying [`SourceRef`] for this scene.
    pub fn as_source(&self) -> SourceRef {
        let ptr = unsafe {
            // as doc said "The scene’s source. Does not increment the reference"
            // we should manually add_ref for it
            obs_scene_get_source(self.inner)
        };
        SourceRef::from_raw(ptr).expect("obs_scene_get_source")
    }

    /// Adds `source` to the scene and returns the resulting scene item.
    ///
    /// # Panics
    ///
    /// Panics if libobs refuses to add the source; see
    /// [`try_add_source`](Self::try_add_source).
    pub fn add_source(&self, source: SourceRef) -> SceneItemRef {
        self.try_add_source(&source).expect("obs_scene_add")
    }

    /// Adds `source` to the scene, returning the new scene item, or `None`
    /// if libobs refuses (for example, when it would create a cycle).
    pub fn try_add_source(&self, source: &SourceRef) -> Option<SceneItemRef> {
        // Docs: "A new scene item for a source within a scene. Does not
        // increment the reference"; from_raw takes one.
        SceneItemRef::from_raw(unsafe { obs_scene_add(self.inner, source.as_ptr_mut()) })
    }

    /// Invokes `callback` for each item in this scene, bottom to top, until
    /// it returns `false` (`obs_scene_enum_items`). Items inside groups are
    /// not visited; enumerate [`SceneItemRef::group_scene`] for those.
    ///
    /// libobs holds the scene's locks throughout. `callback` may enumerate
    /// other scenes and clone items to keep them alive after enumeration,
    /// but must not add or remove items in this scene. A panic stops
    /// enumeration and is resumed after the locks are released.
    pub fn enum_items<F: FnMut(&SceneItemRef) -> bool>(&self, callback: F) {
        let mut state = Enumeration::new(callback);
        unsafe {
            obs_scene_enum_items(
                self.inner,
                Some(enum_items_proc::<F>),
                (&mut state as *mut Enumeration<F>).cast(),
            );
        }
        state.finish();
    }
}

/// Placement of a scene item within its scene (`obs_transform_info`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TransformInfo {
    /// Position of the alignment point, in canvas pixels.
    pub pos: [f32; 2],
    /// Clockwise rotation, in degrees.
    pub rot: f32,
    /// Horizontal and vertical scale factors.
    pub scale: [f32; 2],
    /// `OBS_ALIGN_*` flags selecting the item's anchor point.
    pub alignment: u32,
    /// How the item is fitted into [`bounds`](Self::bounds).
    pub bounds_type: obs_bounds_type,
    /// `OBS_ALIGN_*` flags positioning the item inside its bounds.
    pub bounds_alignment: u32,
    /// Bounding box size, in canvas pixels.
    pub bounds: [f32; 2],
    /// Whether the item is cropped to its bounds.
    pub crop_to_bounds: bool,
}

impl Default for TransformInfo {
    fn default() -> Self {
        Self {
            pos: [0.0; 2],
            rot: 0.0,
            scale: [1.0; 2],
            alignment: 0,
            bounds_type: obs_bounds_type_OBS_BOUNDS_NONE,
            bounds_alignment: 0,
            bounds: [0.0; 2],
            crop_to_bounds: false,
        }
    }
}

fn from_vec2(value: vec2) -> [f32; 2] {
    // Every vec2 union variant is two plain f32 values.
    unsafe { value.__bindgen_anon_1.ptr }
}

fn to_vec2(value: [f32; 2]) -> vec2 {
    let mut raw = vec2::default();
    raw.__bindgen_anon_1.ptr = value;
    raw
}

impl From<obs_transform_info> for TransformInfo {
    fn from(raw: obs_transform_info) -> Self {
        Self {
            pos: from_vec2(raw.pos),
            rot: raw.rot,
            scale: from_vec2(raw.scale),
            alignment: raw.alignment,
            bounds_type: raw.bounds_type,
            bounds_alignment: raw.bounds_alignment,
            bounds: from_vec2(raw.bounds),
            crop_to_bounds: raw.crop_to_bounds,
        }
    }
}

impl From<TransformInfo> for obs_transform_info {
    fn from(info: TransformInfo) -> Self {
        Self {
            pos: to_vec2(info.pos),
            rot: info.rot,
            scale: to_vec2(info.scale),
            alignment: info.alignment,
            bounds_type: info.bounds_type,
            bounds_alignment: info.bounds_alignment,
            bounds: to_vec2(info.bounds),
            crop_to_bounds: info.crop_to_bounds,
        }
    }
}

/// Pixels cropped from each edge of a scene item's source.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SceneItemCrop {
    /// Pixels cropped from the left edge.
    pub left: i32,
    /// Pixels cropped from the top edge.
    pub top: i32,
    /// Pixels cropped from the right edge.
    pub right: i32,
    /// Pixels cropped from the bottom edge.
    pub bottom: i32,
}

/// A reference-counted handle to an OBS scene item (`obs_sceneitem_t`).
///
/// A scene item is the binding of a single [`SourceRef`] into a
/// [`SceneRef`], carrying its position, scale, visibility, and other
/// per-instance state.
pub struct SceneItemRef {
    inner: *mut obs_sceneitem_t,
}

impl_ptr_wrapper!(
    @ptr: inner,
    SceneItemRef,
    obs_sceneitem_t,
    @addref: obs_sceneitem_addref,
    obs_sceneitem_release
);

/// Ends a deferred-update batch even if the batch closure unwinds.
struct DeferredUpdate(*mut obs_sceneitem_t);

impl Drop for DeferredUpdate {
    fn drop(&mut self) {
        unsafe { obs_sceneitem_defer_update_end(self.0) }
    }
}

impl SceneItemRef {
    /// Returns whether the scene item is currently visible.
    pub fn visible(&self) -> bool {
        unsafe { obs_sceneitem_visible(self.inner) }
    }

    /// Returns the item's scene-unique numeric identifier.
    pub fn id(&self) -> i64 {
        unsafe { obs_sceneitem_get_id(self.inner) }
    }

    /// Returns a new reference to the item's source, or `None` if the
    /// source is already being destroyed.
    pub fn source(&self) -> Option<SourceRef> {
        SourceRef::from_raw(unsafe { obs_sceneitem_get_source(self.inner) })
    }

    /// Returns whether the item is locked against user edits in OBS.
    pub fn locked(&self) -> bool {
        unsafe { obs_sceneitem_locked(self.inner) }
    }

    /// Returns whether the item is a group.
    pub fn is_group(&self) -> bool {
        unsafe { obs_sceneitem_is_group(self.inner) }
    }

    /// Returns the scene holding a group item's children, or `None` if the
    /// item is not a group.
    pub fn group_scene(&self) -> Option<SceneRef> {
        SceneRef::from_raw(unsafe { obs_sceneitem_group_get_scene(self.inner) })
    }

    /// Returns the item's full transform (`obs_sceneitem_get_info2`).
    pub fn info(&self) -> TransformInfo {
        let mut raw = obs_transform_info::from(TransformInfo::default());
        unsafe { obs_sceneitem_get_info2(self.inner, &mut raw) };
        raw.into()
    }

    /// Replaces the item's full transform (`obs_sceneitem_set_info2`).
    pub fn set_info(&mut self, info: &TransformInfo) {
        let raw = obs_transform_info::from(*info);
        unsafe { obs_sceneitem_set_info2(self.inner, &raw) }
    }

    /// Returns the item's crop.
    pub fn crop(&self) -> SceneItemCrop {
        let mut raw = obs_sceneitem_crop::default();
        unsafe { obs_sceneitem_get_crop(self.inner, &mut raw) };
        SceneItemCrop {
            left: raw.left,
            top: raw.top,
            right: raw.right,
            bottom: raw.bottom,
        }
    }

    /// Replaces the item's crop.
    pub fn set_crop(&mut self, crop: &SceneItemCrop) {
        let raw = obs_sceneitem_crop {
            left: crop.left,
            top: crop.top,
            right: crop.right,
            bottom: crop.bottom,
        };
        unsafe { obs_sceneitem_set_crop(self.inner, &raw) }
    }

    /// Sets the item's horizontal and vertical scale factors.
    pub fn set_scale(&mut self, scale: [f32; 2]) {
        unsafe { obs_sceneitem_set_scale(self.inner, &to_vec2(scale)) }
    }

    /// Runs `update` with transform recalculation deferred until it
    /// returns, so several edits apply as one change
    /// (`obs_sceneitem_defer_update_begin`/`_end`).
    pub fn defer_update<R>(&mut self, update: impl FnOnce(&mut Self) -> R) -> R {
        unsafe { obs_sceneitem_defer_update_begin(self.inner) };
        let _batch = DeferredUpdate(self.inner);
        update(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transform_info_round_trips_through_the_c_layout() {
        let info = TransformInfo {
            pos: [12.5, -3.0],
            rot: 90.0,
            scale: [0.5, 2.0],
            alignment: obs_sys_rs::OBS_ALIGN_LEFT | obs_sys_rs::OBS_ALIGN_TOP,
            bounds_type: obs_sys_rs::obs_bounds_type_OBS_BOUNDS_SCALE_INNER,
            bounds_alignment: obs_sys_rs::OBS_ALIGN_RIGHT,
            bounds: [640.0, 360.0],
            crop_to_bounds: true,
        };
        let raw = obs_transform_info::from(info);
        let vec2 = |value: vec2| unsafe { value.__bindgen_anon_1.__bindgen_anon_1 };
        assert_eq!((vec2(raw.pos).x, vec2(raw.pos).y), (12.5, -3.0));
        assert_eq!((vec2(raw.scale).x, vec2(raw.scale).y), (0.5, 2.0));
        assert_eq!((vec2(raw.bounds).x, vec2(raw.bounds).y), (640.0, 360.0));
        assert_eq!(TransformInfo::from(raw), info);
    }
}

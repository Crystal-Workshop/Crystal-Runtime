use std::sync::Arc;

use glam::Vec3;
use parking_lot::RwLock;

use crate::scene::SceneObject;

/// Thread-safe container mirroring the mutable state of the scene graph.
#[derive(Debug, Default)]
pub struct DataModel {
    objects: Arc<RwLock<Vec<SceneObject>>>,
}

impl Clone for DataModel {
    fn clone(&self) -> Self {
        Self {
            objects: Arc::clone(&self.objects),
        }
    }
}

impl DataModel {
    /// Creates an empty data model.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a data model from an initial set of objects.
    pub fn from_objects(objects: Vec<SceneObject>) -> Self {
        Self {
            objects: Arc::new(RwLock::new(objects)),
        }
    }

    /// Replaces the stored objects with a new snapshot.
    pub fn replace_objects(&self, objects: Vec<SceneObject>) {
        *self.objects.write() = objects;
    }

    /// Returns a snapshot of all stored objects.
    pub fn all_objects(&self) -> Vec<SceneObject> {
        self.objects.read().clone()
    }

    /// Returns a clone of the requested object.
    pub fn get(&self, name: &str) -> Option<SceneObject> {
        self.objects
            .read()
            .iter()
            .find(|object| object.name == name)
            .cloned()
    }

    /// Applies a mutation to the requested object.
    pub fn update<F, R>(&self, name: &str, mut updater: F) -> Option<R>
    where
        F: FnMut(&mut SceneObject) -> R,
    {
        let mut guard = self.objects.write();
        let object = guard.iter_mut().find(|object| object.name == name)?;
        Some(updater(object))
    }

    pub fn set_position(&self, name: &str, position: Vec3) -> bool {
        self.update(name, |obj| obj.position = position).is_some()
    }

    pub fn set_rotation(&self, name: &str, rotation: Vec3) -> bool {
        self.update(name, |obj| obj.rotation = rotation).is_some()
    }

    pub fn set_scale(&self, name: &str, scale: Vec3) -> bool {
        self.update(name, |obj| obj.scale = scale).is_some()
    }

    pub fn set_color(&self, name: &str, color: Vec3) -> bool {
        self.update(name, |obj| obj.color = color).is_some()
    }

    pub fn set_fov(&self, name: &str, fov: f32) -> bool {
        self.update(name, |obj| obj.fov = fov).is_some()
    }

    pub fn set_intensity(&self, name: &str, intensity: f32) -> bool {
        self.update(name, |obj| obj.intensity = intensity).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::SceneObject;

    fn make_object(name: &str) -> SceneObject {
        SceneObject {
            name: name.to_string(),
            ..SceneObject::default()
        }
    }

    #[test]
    fn replace_and_get_object() {
        let model = DataModel::from_objects(vec![make_object("Cube")]);
        assert!(model.get("Cube").is_some());
        model.replace_objects(vec![make_object("Sphere")]);
        assert!(model.get("Cube").is_none());
        assert!(model.get("Sphere").is_some());
    }

    #[test]
    fn update_modifies_object() {
        let model = DataModel::from_objects(vec![make_object("Camera")]);
        model.set_fov("Camera", 60.0);
        let cam = model.get("Camera").unwrap();
        assert_eq!(cam.fov, 60.0);
    }

    #[test]
    fn update_returns_false_for_missing_object() {
        let model = DataModel::new();
        assert!(!model.set_color("Unknown", Vec3::ONE));
    }
}

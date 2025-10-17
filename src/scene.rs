use anyhow::{anyhow, Context, Result};
use glam::Vec3;
use roxmltree::{Document, Node};
use serde::{Deserialize, Serialize};

/// Runtime representation of a scene.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Scene {
    pub objects: Vec<SceneObject>,
    pub lights: Vec<Light>,
}

impl Scene {
    /// Parses the scene XML produced by the authoring tools.
    pub fn from_xml(xml: &str) -> Result<Self> {
        let document = Document::parse(xml).context("invalid scene XML")?;
        let mut objects = Vec::new();

        for node in document.descendants().filter(|n| n.has_tag_name("object")) {
            let mut object = SceneObject::default();
            object.name = required_text(&node, "name")?;
            object.object_type = optional_text(&node, "type").unwrap_or_else(|| "mesh".to_string());
            object.mesh = optional_text(&node, "mesh");
            object.color = parse_color(optional_text(&node, "color"), object.color)?;
            object.position = parse_vec3(optional_text(&node, "position"), object.position)?;
            object.rotation = parse_vec3(optional_text(&node, "rotation"), object.rotation)?;
            object.scale = parse_vec3(optional_text(&node, "scale"), object.scale)?;
            object.fov = parse_f32(optional_text(&node, "fov"), object.fov)?;
            object.intensity = parse_f32(optional_text(&node, "intensity"), object.intensity)?;
            objects.push(object);
        }

        let lights = objects
            .iter()
            .filter(|obj| obj.object_type == "light")
            .map(|obj| Light {
                position: obj.position,
                color: obj.color,
                intensity: obj.intensity,
            })
            .collect();

        Ok(Self { objects, lights })
    }
}

/// Scene object as described by the authoring tools.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneObject {
    pub name: String,
    #[serde(rename = "type")]
    pub object_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mesh: Option<String>,
    #[serde(default = "default_color")]
    pub color: Vec3,
    #[serde(default)]
    pub position: Vec3,
    #[serde(default)]
    pub rotation: Vec3,
    #[serde(default = "default_scale")]
    pub scale: Vec3,
    #[serde(default = "default_fov")]
    pub fov: f32,
    #[serde(default = "default_intensity")]
    pub intensity: f32,
}

impl Default for SceneObject {
    fn default() -> Self {
        Self {
            name: String::new(),
            object_type: String::new(),
            mesh: None,
            color: default_color(),
            position: Vec3::ZERO,
            rotation: Vec3::ZERO,
            scale: Vec3::ONE,
            fov: default_fov(),
            intensity: default_intensity(),
        }
    }
}

fn default_color() -> Vec3 {
    Vec3::ONE
}

fn default_scale() -> Vec3 {
    Vec3::ONE
}

fn default_fov() -> f32 {
    45.0
}

fn default_intensity() -> f32 {
    1.0
}

/// Light extracted from the scene object list.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Light {
    pub position: Vec3,
    pub color: Vec3,
    pub intensity: f32,
}

fn required_text(node: &Node<'_, '_>, tag: &str) -> Result<String> {
    optional_text(node, tag).ok_or_else(|| anyhow!("<{tag}> tag is missing"))
}

fn optional_text(node: &Node<'_, '_>, tag: &str) -> Option<String> {
    node.children()
        .find(|child| child.has_tag_name(tag))
        .and_then(|child| child.text())
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(|text| text.to_string())
}

fn parse_vec3(value: Option<String>, default: Vec3) -> Result<Vec3> {
    let Some(value) = value else {
        return Ok(default);
    };
    let mut numbers = value
        .split_whitespace()
        .filter_map(|component| component.parse::<f32>().ok());
    let x = numbers
        .next()
        .ok_or_else(|| anyhow!("vector is missing components"))?;
    let y = numbers
        .next()
        .ok_or_else(|| anyhow!("vector is missing components"))?;
    let z = numbers
        .next()
        .ok_or_else(|| anyhow!("vector is missing components"))?;
    Ok(Vec3::new(x, y, z))
}

fn parse_color(value: Option<String>, default: Vec3) -> Result<Vec3> {
    let Some(value) = value else {
        return Ok(default);
    };
    let mut numbers = value
        .split_whitespace()
        .filter_map(|component| component.parse::<f32>().ok());
    let r = numbers
        .next()
        .ok_or_else(|| anyhow!("color is missing components"))?;
    let g = numbers
        .next()
        .ok_or_else(|| anyhow!("color is missing components"))?;
    let b = numbers
        .next()
        .ok_or_else(|| anyhow!("color is missing components"))?;
    Ok(Vec3::new(r / 255.0, g / 255.0, b / 255.0))
}

fn parse_f32(value: Option<String>, default: f32) -> Result<f32> {
    match value {
        Some(value) => value
            .parse::<f32>()
            .map_err(|err| anyhow!("failed to parse float: {err}")),
        None => Ok(default),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
    <scene>
        <object>
            <name>Camera</name>
            <type>camera</type>
            <fov>90</fov>
        </object>
        <object>
            <name>Light</name>
            <type>light</type>
            <intensity>2.5</intensity>
            <position>0 5 0</position>
            <color>255 128 0</color>
        </object>
    </scene>
    "#;

    #[test]
    fn parse_scene_populates_objects_and_lights() {
        let scene = Scene::from_xml(SAMPLE).unwrap();
        assert_eq!(scene.objects.len(), 2);
        let camera = scene.objects.iter().find(|o| o.name == "Camera").unwrap();
        assert_eq!(camera.object_type, "camera");
        assert_eq!(camera.fov, 90.0);
        assert_eq!(scene.lights.len(), 1);
        let light = scene.lights[0];
        assert_eq!(light.position, Vec3::new(0.0, 5.0, 0.0));
        assert!((light.intensity - 2.5).abs() < f32::EPSILON);
        assert_eq!(light.color, Vec3::new(1.0, 128.0 / 255.0, 0.0));
    }

    #[test]
    fn missing_name_is_an_error() {
        let bad = "<scene><object><type>mesh</type></object></scene>";
        assert!(Scene::from_xml(bad).is_err());
    }
}

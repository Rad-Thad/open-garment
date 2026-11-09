use pyo3::buffer::PyBuffer;
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict};

mod coloring;
pub mod engine;
mod grid;

use engine::{Solver as InnerSolver, SolverParams};

const PARAM_KEYS: [&str; 15] = [
    "frame_time",
    "elapsed",
    "seam_seconds",
    "gravity_x",
    "gravity_y",
    "gravity_z",
    "damping",
    "body_margin",
    "body_grip",
    "body_step",
    "body_band",
    "self_thickness",
    "weld_reach",
    "weld_gain",
    "max_step",
];

fn f32_array(obj: &Bound<'_, PyAny>, name: &str) -> PyResult<Vec<f32>> {
    let buf = PyBuffer::<f32>::get(obj).map_err(|_| {
        PyValueError::new_err(format!("{name} must be a little-endian C-contiguous float32 array"))
    })?;
    if !buf.is_c_contiguous() {
        return Err(PyValueError::new_err(format!("{name} must be C-contiguous")));
    }
    buf.to_vec(obj.py())
}

fn u32_array(obj: &Bound<'_, PyAny>, name: &str) -> PyResult<Vec<u32>> {
    let buf = PyBuffer::<u32>::get(obj).map_err(|_| {
        PyValueError::new_err(format!("{name} must be a little-endian C-contiguous uint32 array"))
    })?;
    if !buf.is_c_contiguous() {
        return Err(PyValueError::new_err(format!("{name} must be C-contiguous")));
    }
    buf.to_vec(obj.py())
}

fn i32_array(obj: &Bound<'_, PyAny>, name: &str) -> PyResult<Vec<i32>> {
    let buf = PyBuffer::<i32>::get(obj).map_err(|_| {
        PyValueError::new_err(format!("{name} must be a little-endian C-contiguous int32 array"))
    })?;
    if !buf.is_c_contiguous() {
        return Err(PyValueError::new_err(format!("{name} must be C-contiguous")));
    }
    buf.to_vec(obj.py())
}

fn u8_array(obj: &Bound<'_, PyAny>, name: &str) -> PyResult<Vec<u8>> {
    let buf = PyBuffer::<u8>::get(obj).map_err(|_| {
        PyValueError::new_err(format!("{name} must be a little-endian C-contiguous uint8 array"))
    })?;
    if !buf.is_c_contiguous() {
        return Err(PyValueError::new_err(format!("{name} must be C-contiguous")));
    }
    buf.to_vec(obj.py())
}

fn check_len(actual: usize, expected: usize, name: &str) -> PyResult<()> {
    if actual != expected {
        return Err(PyValueError::new_err(format!(
            "{name} has length {actual}, expected {expected}"
        )));
    }
    Ok(())
}

fn apply_params(p: &mut SolverParams, dict: &Bound<'_, PyDict>, required: bool) -> PyResult<()> {
    for (key, value) in dict.iter() {
        let key: String = key.extract()?;
        if !PARAM_KEYS.contains(&key.as_str()) {
            return Err(PyValueError::new_err(format!("unknown param: {key}")));
        }
        let v = value.extract::<f64>()? as f32;
        match key.as_str() {
            "frame_time" => p.frame_time = v,
            "elapsed" => p.elapsed = v,
            "seam_seconds" => p.seam_seconds = v,
            "gravity_x" => p.gravity[0] = v,
            "gravity_y" => p.gravity[1] = v,
            "gravity_z" => p.gravity[2] = v,
            "damping" => p.damping = v,
            "body_margin" => p.body_margin = v,
            "body_grip" => p.body_grip = v,
            "body_step" => p.body_step = v,
            "body_band" => p.body_band = v,
            "self_thickness" => p.self_thickness = v,
            "weld_reach" => p.weld_reach = v,
            "weld_gain" => p.weld_gain = v,
            "max_step" => p.max_step = v,
            _ => unreachable!(),
        }
    }
    if required {
        for key in PARAM_KEYS {
            if !dict.contains(key)? {
                return Err(PyValueError::new_err(format!("missing param: {key}")));
            }
        }
    }
    Ok(())
}

#[pyfunction]
fn adapter_name() -> PyResult<String> {
    InnerSolver::adapter_name()
        .map_err(|e| PyRuntimeError::new_err(format!("No GPU adapter: {e}")))
}

#[pyclass]
struct Solver {
    inner: InnerSolver,
}

#[pymethods]
impl Solver {
    #[new]
    #[pyo3(signature = (positions, inverse_mass, pairs, rest, rest_start, compliance, triangles, rest_positions, partner, contact_weight, params))]
    fn new(
        positions: &Bound<'_, PyAny>,
        inverse_mass: &Bound<'_, PyAny>,
        pairs: &Bound<'_, PyAny>,
        rest: &Bound<'_, PyAny>,
        rest_start: &Bound<'_, PyAny>,
        compliance: &Bound<'_, PyAny>,
        triangles: &Bound<'_, PyAny>,
        rest_positions: &Bound<'_, PyAny>,
        partner: &Bound<'_, PyAny>,
        contact_weight: &Bound<'_, PyAny>,
        params: &Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        let positions = f32_array(positions, "positions")?;
        let n = positions.len() / 3;
        if n == 0 {
            return Err(PyValueError::new_err("positions must contain at least one vertex"));
        }
        let inverse_mass = f32_array(inverse_mass, "inverse_mass")?;
        check_len(inverse_mass.len(), n, "inverse_mass")?;
        let pairs = u32_array(pairs, "pairs")?;
        let m = pairs.len() / 2;
        if m == 0 {
            return Err(PyValueError::new_err("pairs must contain at least one constraint"));
        }
        if pairs.len() % 2 != 0 {
            return Err(PyValueError::new_err("pairs length must be even"));
        }
        let rest = f32_array(rest, "rest")?;
        check_len(rest.len(), m, "rest")?;
        let rest_start = f32_array(rest_start, "rest_start")?;
        check_len(rest_start.len(), m, "rest_start")?;
        let compliance = f32_array(compliance, "compliance")?;
        check_len(compliance.len(), m, "compliance")?;
        let triangles = u32_array(triangles, "triangles")?;
        if triangles.len() % 3 != 0 {
            return Err(PyValueError::new_err("triangles length must be a multiple of 3"));
        }
        let rest_positions = f32_array(rest_positions, "rest_positions")?;
        check_len(rest_positions.len(), n * 3, "rest_positions")?;
        let partner = i32_array(partner, "partner")?;
        check_len(partner.len(), n, "partner")?;
        let contact_weight = f32_array(contact_weight, "contact_weight")?;
        check_len(contact_weight.len(), n, "contact_weight")?;
        let params_dict = params.cast::<PyDict>()?;
        let mut p = SolverParams::default();
        apply_params(&mut p, params_dict, true)?;
        let inner = InnerSolver::new(
            &positions,
            &inverse_mass,
            &pairs,
            &rest,
            &rest_start,
            &compliance,
            &triangles,
            &rest_positions,
            &partner,
            &contact_weight,
            &p,
        )
        .map_err(|e| {
            if e.starts_with("No GPU adapter") {
                PyRuntimeError::new_err(e)
            } else {
                PyValueError::new_err(e)
            }
        })?;
        Ok(Solver { inner })
    }

    fn set_collider(&mut self, positions: &Bound<'_, PyAny>, triangles: &Bound<'_, PyAny>) -> PyResult<()> {
        let positions = f32_array(positions, "positions")?;
        if positions.len() % 3 != 0 {
            return Err(PyValueError::new_err("positions length must be a multiple of 3"));
        }
        let triangles = u32_array(triangles, "triangles")?;
        if triangles.len() % 3 != 0 {
            return Err(PyValueError::new_err("triangles length must be a multiple of 3"));
        }
        self.inner.set_collider(&positions, &triangles);
        Ok(())
    }

    fn set_pins(&mut self, mask: &Bound<'_, PyAny>, targets: &Bound<'_, PyAny>) -> PyResult<()> {
        let mask = u8_array(mask, "mask")?;
        let n = mask.len();
        let targets = f32_array(targets, "targets")?;
        check_len(targets.len(), n * 3, "targets")?;
        self.inner.set_pins(&mask, &targets);
        Ok(())
    }

    fn set_params(&mut self, params: &Bound<'_, PyAny>) -> PyResult<()> {
        let dict = params.cast::<PyDict>()?;
        let mut p = *self.inner.params();
        apply_params(&mut p, dict, false)?;
        self.inner.set_params(p);
        Ok(())
    }

    fn step(&mut self, substeps: usize, iterations: usize) -> PyResult<()> {
        self.inner
            .step(substeps as u32, iterations as u32)
            .map_err(|e| {
                if e == "GPU device lost" {
                    PyRuntimeError::new_err(e)
                } else {
                    PyValueError::new_err(e)
                }
            })
    }

    fn positions(&self, py: Python<'_>) -> Py<PyBytes> {
        let data = self.inner.positions();
        PyBytes::new(py, bytemuck::cast_slice(&data)).unbind()
    }
}

#[pymodule]
fn garment_solver(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(adapter_name, m)?)?;
    m.add_class::<Solver>()?;
    Ok(())
}

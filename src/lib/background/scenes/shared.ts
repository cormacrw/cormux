import * as THREE from 'three'
import { RoomEnvironment } from 'three/examples/jsm/environments/RoomEnvironment.js'

/** Scene clear colours; they match the page background in index.html. */
export const DARK_BG = '#0b0c0e'
export const LIGHT_BG = '#eef0f3'

export function backgroundColor(dark: boolean) {
  return dark ? DARK_BG : LIGHT_BG
}

/** Free every geometry, material and texture under `root`. */
export function disposeObject(root: THREE.Object3D) {
  root.traverse((node) => {
    const mesh = node as THREE.Mesh
    mesh.geometry?.dispose()
    const material = mesh.material
    if (!material) return
    for (const one of Array.isArray(material) ? material : [material]) {
      for (const value of Object.values(one)) {
        if (value instanceof THREE.Texture) value.dispose()
      }
      if ('uniforms' in one) {
        for (const uniform of Object.values(
          (one as THREE.ShaderMaterial).uniforms,
        )) {
          if (uniform.value instanceof THREE.Texture) uniform.value.dispose()
        }
      }
      one.dispose()
    }
  })
}

/**
 * A soft studio environment map for reflections, baked once with PMREM.
 * Dispose the returned texture with the scene.
 */
export function studioEnvironment(renderer: THREE.WebGLRenderer) {
  const pmrem = new THREE.PMREMGenerator(renderer)
  const room = new RoomEnvironment()
  const texture = pmrem.fromScene(room, 0.04).texture
  room.dispose()
  pmrem.dispose()
  return texture
}

/** A radial falloff texture for fake soft contact shadows and glows. */
export function radialTexture(size = 128) {
  const canvas = document.createElement('canvas')
  canvas.width = canvas.height = size
  const context = canvas.getContext('2d')!
  const gradient = context.createRadialGradient(
    size / 2,
    size / 2,
    0,
    size / 2,
    size / 2,
    size / 2,
  )
  gradient.addColorStop(0, 'rgba(255,255,255,1)')
  gradient.addColorStop(0.5, 'rgba(255,255,255,0.45)')
  gradient.addColorStop(1, 'rgba(255,255,255,0)')
  context.fillStyle = gradient
  context.fillRect(0, 0, size, size)
  const texture = new THREE.CanvasTexture(canvas)
  texture.colorSpace = THREE.SRGBColorSpace
  return texture
}

/**
 * Full-screen triangle for shader backgrounds. Its vertex shader ignores the
 * camera, so any camera works.
 */
export function fullScreenTriangle() {
  const geometry = new THREE.BufferGeometry()
  geometry.setAttribute(
    'position',
    new THREE.Float32BufferAttribute([-1, -1, 0, 3, -1, 0, -1, 3, 0], 3),
  )
  geometry.setAttribute(
    'uv',
    new THREE.Float32BufferAttribute([0, 0, 2, 0, 0, 2], 2),
  )
  return geometry
}

export const FULL_SCREEN_VERTEX = /* glsl */ `
  varying vec2 vUv;
  void main() {
    vUv = uv;
    gl_Position = vec4(position.xy, 0.0, 1.0);
  }
`

/** Cheap 2D value noise and fbm, shared by the shader scenes. */
export const NOISE_GLSL = /* glsl */ `
  float hash(vec2 p) {
    p = fract(p * vec2(123.34, 456.21));
    p += dot(p, p + 45.32);
    return fract(p.x * p.y);
  }
  float noise(vec2 p) {
    vec2 i = floor(p);
    vec2 f = fract(p);
    vec2 u = f * f * (3.0 - 2.0 * f);
    return mix(
      mix(hash(i), hash(i + vec2(1.0, 0.0)), u.x),
      mix(hash(i + vec2(0.0, 1.0)), hash(i + vec2(1.0, 1.0)), u.x),
      u.y
    );
  }
  float fbm(vec2 p) {
    float value = 0.0;
    float amplitude = 0.5;
    for (int i = 0; i < 4; i++) {
      value += amplitude * noise(p);
      p *= 2.03;
      amplitude *= 0.5;
    }
    return value;
  }
`

/** Perspective camera helper that keeps aspect in sync on resize. */
export function perspective(
  fov: number,
  width: number,
  height: number,
  near = 0.1,
  far = 100,
) {
  return new THREE.PerspectiveCamera(
    fov,
    width / Math.max(1, height),
    near,
    far,
  )
}

export function resizePerspective(
  camera: THREE.PerspectiveCamera,
  width: number,
  height: number,
) {
  camera.aspect = width / Math.max(1, height)
  camera.updateProjectionMatrix()
}

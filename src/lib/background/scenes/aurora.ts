import * as THREE from 'three'
import type { CreateScene } from '../types'
import {
  FULL_SCREEN_VERTEX,
  NOISE_GLSL,
  backgroundColor,
  disposeObject,
  fullScreenTriangle,
} from './shared'

const fragmentShader = /* glsl */ `
  uniform vec3 uAccent;
  uniform vec3 uBg;
  uniform float uTime;
  uniform float uDark;
  uniform float uAspect;
  varying vec2 vUv;
  ${NOISE_GLSL}

  float band(vec2 p, float offset, float t) {
    float wave = fbm(vec2(p.x * 1.2 + t * 0.05 + offset, t * 0.03 + offset)) * 0.9;
    float y = 0.55 + offset * 0.12 + wave - 0.45;
    float d = abs(p.y - y);
    return exp(-d * d * 38.0);
  }

  void main() {
    vec2 p = vec2(vUv.x * uAspect, vUv.y);
    float t = uTime;
    float a = band(p, 0.0, t);
    float b = band(p + vec2(3.1, 0.0), 1.0, t * 0.8);
    float c = band(p + vec2(7.4, 0.0), -1.3, t * 1.2);
    float shimmer = 0.75 + 0.25 * fbm(vec2(p.x * 6.0 + t * 0.1, p.y * 2.0));

    vec3 hueShift = vec3(uAccent.g, uAccent.b, uAccent.r);
    vec3 second = mix(uAccent, hueShift, 0.35);
    vec3 third = mix(uAccent, vec3(1.0), 0.45);
    float strength = mix(0.55, 0.6, uDark);

    vec3 color = uBg;
    color = mix(color, uAccent, a * shimmer * strength);
    color = mix(color, second, b * shimmer * strength * 0.7);
    color = mix(color, third, c * shimmer * strength * 0.45);
    // Gentle vignette toward the background.
    float vignette = smoothstep(1.25, 0.25, length(vUv - vec2(0.5, 0.55)));
    color = mix(uBg, color, vignette);
    gl_FragColor = vec4(color, 1.0);
    #include <colorspace_fragment>
  }
`

/** Soft flowing bands of accent colour across the window. */
export const create: CreateScene = ({
  renderer,
  width,
  height,
  accent,
  dark,
}) => {
  const scene = new THREE.Scene()
  const camera = new THREE.OrthographicCamera(-1, 1, 1, -1, 0, 1)
  const uniforms = {
    uAccent: { value: new THREE.Color(accent) },
    uBg: { value: new THREE.Color(backgroundColor(dark)) },
    uTime: { value: 0 },
    uDark: { value: dark ? 1 : 0 },
    uAspect: { value: width / Math.max(1, height) },
  }
  const mesh = new THREE.Mesh(
    fullScreenTriangle(),
    new THREE.ShaderMaterial({
      uniforms,
      vertexShader: FULL_SCREEN_VERTEX,
      fragmentShader,
      depthTest: false,
      depthWrite: false,
      toneMapped: false,
    }),
  )
  mesh.frustumCulled = false
  scene.add(mesh)

  return {
    setAccent: (hex) => void uniforms.uAccent.value.set(hex),
    setTheme(isDark) {
      uniforms.uBg.value.set(backgroundColor(isDark))
      uniforms.uDark.value = isDark ? 1 : 0
    },
    resize(w, h) {
      uniforms.uAspect.value = w / Math.max(1, h)
    },
    render(time) {
      uniforms.uTime.value = time
      renderer.render(scene, camera)
    },
    dispose() {
      disposeObject(scene)
    },
  }
}

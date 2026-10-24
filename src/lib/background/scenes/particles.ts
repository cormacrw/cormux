import * as THREE from 'three'
import type { CreateScene } from '../types'
import {
  backgroundColor,
  disposeObject,
  perspective,
  resizePerspective,
} from './shared'

const COUNT = 7000

const vertexShader = /* glsl */ `
  uniform float uPixelRatio;
  uniform float uTime;
  attribute float aSeed;
  varying float vSeed;
  varying float vDepth;

  void main() {
    vec3 p = position;
    p.y += sin(uTime * 0.3 + aSeed * 40.0) * 0.06;
    vec4 view = modelViewMatrix * vec4(p, 1.0);
    vSeed = aSeed;
    vDepth = -view.z;
    gl_Position = projectionMatrix * view;
    gl_PointSize = (2.0 + aSeed * 3.5) * uPixelRatio * (8.0 / -view.z);
  }
`

const fragmentShader = /* glsl */ `
  uniform vec3 uAccent;
  uniform vec3 uBg;
  uniform float uDark;
  varying float vSeed;
  varying float vDepth;

  void main() {
    float d = length(gl_PointCoord - 0.5);
    float alpha = smoothstep(0.5, 0.0, d);
    vec3 tint = mix(uAccent, vec3(1.0), step(0.8, vSeed) * mix(0.25, 0.6, uDark));
    float fade = 1.0 - smoothstep(10.0, 22.0, vDepth);
    alpha *= fade * mix(0.55, 0.8, uDark);
    gl_FragColor = vec4(tint, alpha);
    #include <colorspace_fragment>
  }
`

/** A slowly turning spiral galaxy of accent-coloured points. */
export const create: CreateScene = ({
  renderer,
  width,
  height,
  accent,
  dark,
}) => {
  const scene = new THREE.Scene()
  const background = new THREE.Color()
  scene.background = background

  const camera = perspective(50, width, height, 0.1, 60)
  camera.position.set(0, 4.2, 9.5)
  camera.lookAt(0, -0.5, 0)

  // A deterministic galaxy: three arms with scatter, plus a faint halo.
  const positions = new Float32Array(COUNT * 3)
  const seeds = new Float32Array(COUNT)
  let state = 1
  const random = () => {
    state = (state * 16807) % 2147483647
    return state / 2147483647
  }
  for (let i = 0; i < COUNT; i++) {
    const halo = i % 7 === 0
    const radius = halo ? 2 + random() * 12 : Math.pow(random(), 1.6) * 9 + 0.3
    const arm = ((i % 3) / 3) * Math.PI * 2
    const angle = halo ? random() * Math.PI * 2 : arm + radius * 0.42
    const scatter = (halo ? 2.5 : 0.35 + radius * 0.07) * (random() - 0.5) * 2
    positions[i * 3] = Math.cos(angle) * radius + scatter
    positions[i * 3 + 1] = (random() - 0.5) * (halo ? 5 : 0.5)
    positions[i * 3 + 2] = Math.sin(angle) * radius + scatter * 0.7
    seeds[i] = random()
  }
  const geometry = new THREE.BufferGeometry()
  geometry.setAttribute('position', new THREE.BufferAttribute(positions, 3))
  geometry.setAttribute('aSeed', new THREE.BufferAttribute(seeds, 1))

  const uniforms = {
    uAccent: { value: new THREE.Color(accent) },
    uBg: { value: background },
    uDark: { value: dark ? 1 : 0 },
    uTime: { value: 0 },
    uPixelRatio: { value: renderer.getPixelRatio() },
  }
  const material = new THREE.ShaderMaterial({
    uniforms,
    vertexShader,
    fragmentShader,
    transparent: true,
    depthWrite: false,
    toneMapped: false,
  })
  const points = new THREE.Points(geometry, material)
  points.rotation.z = 0.12
  scene.add(points)

  function setTheme(isDark: boolean) {
    background.set(backgroundColor(isDark))
    uniforms.uDark.value = isDark ? 1 : 0
    material.blending = isDark ? THREE.AdditiveBlending : THREE.NormalBlending
    material.needsUpdate = true
  }

  setTheme(dark)

  return {
    setAccent: (hex) => void uniforms.uAccent.value.set(hex),
    setTheme,
    resize(w, h) {
      resizePerspective(camera, w, h)
      uniforms.uPixelRatio.value = renderer.getPixelRatio()
    },
    render(time) {
      uniforms.uTime.value = time
      points.rotation.y = time * 0.02
      renderer.render(scene, camera)
    },
    dispose() {
      disposeObject(scene)
    },
  }
}

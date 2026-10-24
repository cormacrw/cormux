import * as THREE from 'three'
import type { CreateScene } from '../types'
import {
  backgroundColor,
  disposeObject,
  perspective,
  resizePerspective,
} from './shared'

const vertexShader = /* glsl */ `
  uniform float uTime;
  varying vec3 vWorld;
  varying float vHeight;

  float waves(vec2 p, float t) {
    float h = sin(p.x * 0.35 + t * 0.35) * 0.45;
    h += sin(p.y * 0.42 - t * 0.28 + p.x * 0.12) * 0.35;
    h += sin((p.x + p.y) * 0.8 + t * 0.5) * 0.12;
    h += sin(length(p - vec2(4.0, -6.0)) * 0.9 - t * 0.6) * 0.08;
    return h;
  }

  void main() {
    vec3 p = position;
    float h = waves(p.xy, uTime);
    p.z += h;
    vHeight = h;
    vec4 world = modelMatrix * vec4(p, 1.0);
    vWorld = world.xyz;
    gl_Position = projectionMatrix * viewMatrix * world;
  }
`

const fragmentShader = /* glsl */ `
  uniform vec3 uAccent;
  uniform vec3 uBg;
  uniform float uDark;
  uniform vec3 uCamera;
  varying vec3 vWorld;
  varying float vHeight;

  void main() {
    vec3 normal = normalize(cross(dFdx(vWorld), dFdy(vWorld)));
    if (normal.y < 0.0) normal = -normal;
    vec3 light = normalize(vec3(-0.4, 0.8, 0.3));
    vec3 view = normalize(uCamera - vWorld);
    float diffuse = clamp(dot(normal, light), 0.0, 1.0);
    float fresnel = pow(1.0 - clamp(dot(normal, view), 0.0, 1.0), 3.0);
    vec3 halfway = normalize(light + view);
    float spec = pow(clamp(dot(normal, halfway), 0.0, 1.0), 60.0);

    float lift = smoothstep(-0.9, 0.9, vHeight);
    vec3 deep = mix(uBg, uAccent, mix(0.3, 0.12, uDark));
    vec3 crest = mix(uBg, uAccent, mix(0.75, 0.55, uDark));
    vec3 color = mix(deep, crest, lift * 0.6 + diffuse * 0.3);
    color += fresnel * mix(0.12, 0.25, uDark) * mix(uAccent, vec3(1.0), 0.5);
    color += spec * mix(0.25, 0.35, uDark);

    float dist = length(uCamera - vWorld);
    float fog = smoothstep(6.0, 21.0, dist);
    gl_FragColor = vec4(mix(color, uBg, fog), 1.0);
    #include <colorspace_fragment>
  }
`

/** A slow, rolling accent-coloured surface. */
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

  const camera = perspective(45, width, height, 0.1, 60)
  camera.position.set(0, 3.2, 9)
  camera.lookAt(0, 0, -3)

  const uniforms = {
    uTime: { value: 0 },
    uAccent: { value: new THREE.Color(accent) },
    uBg: { value: background },
    uDark: { value: dark ? 1 : 0 },
    uCamera: { value: camera.position },
  }
  const material = new THREE.ShaderMaterial({
    uniforms,
    vertexShader,
    fragmentShader,
    toneMapped: false,
  })
  const plane = new THREE.Mesh(
    new THREE.PlaneGeometry(48, 40, 180, 150),
    material,
  )
  plane.rotation.x = -Math.PI / 2
  plane.position.set(0, -1.2, -6)
  scene.add(plane)

  function setTheme(isDark: boolean) {
    background.set(backgroundColor(isDark))
    uniforms.uDark.value = isDark ? 1 : 0
  }

  setTheme(dark)

  return {
    setAccent: (hex) => void uniforms.uAccent.value.set(hex),
    setTheme,
    resize: (w, h) => resizePerspective(camera, w, h),
    render(time) {
      uniforms.uTime.value = time
      renderer.render(scene, camera)
    },
    dispose() {
      disposeObject(scene)
    },
  }
}

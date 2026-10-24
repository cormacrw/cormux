import * as THREE from 'three'
import type { CreateScene } from '../types'
import {
  backgroundColor,
  disposeObject,
  perspective,
  resizePerspective,
  studioEnvironment,
} from './shared'

// x, y, z, scale, shape (0 icosahedron, 1 octahedron), accent (0 deep, 1 pale)
const LAYOUT: [number, number, number, number, 0 | 1, 0 | 1][] = [
  [-3.4, 0.8, -1, 1.25, 0, 0],
  [3.1, -0.4, -0.4, 1.05, 1, 0],
  [0.2, 1.6, -3.4, 0.8, 1, 1],
  [-1.2, -1.6, 0.8, 0.6, 0, 1],
  [1.7, 1.3, 1.2, 0.45, 0, 0],
  [5.2, 1.9, -3.6, 0.9, 0, 1],
  [-5, -1.6, -3, 0.85, 1, 0],
]

/** Faceted icosahedra and octahedra turning slowly. */
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
  const environment = studioEnvironment(renderer)
  scene.environment = environment

  const camera = perspective(40, width, height, 0.1, 60)
  camera.position.set(0, 0, 10)

  const key = new THREE.DirectionalLight(0xffffff, 1.4)
  key.position.set(-4, 6, 5)
  const rim = new THREE.DirectionalLight(accent, 1.2)
  rim.position.set(4, -2, -4)
  scene.add(key, rim)

  const deep = new THREE.MeshPhysicalMaterial({
    color: accent,
    metalness: 0.25,
    roughness: 0.2,
    clearcoat: 1,
    flatShading: true,
  })
  const pale = new THREE.MeshPhysicalMaterial({
    color: accent,
    metalness: 0.05,
    roughness: 0.3,
    clearcoat: 0.8,
    flatShading: true,
    iridescence: 0.4,
  })
  const geometries = [
    new THREE.IcosahedronGeometry(1, 0),
    new THREE.OctahedronGeometry(1, 0),
  ]
  const crystals = LAYOUT.map(([x, y, z, s, shape, tone], index) => {
    const mesh = new THREE.Mesh(geometries[shape], tone ? pale : deep)
    mesh.position.set(x, y, z)
    mesh.scale.setScalar(s)
    mesh.rotation.set(index * 0.7, index * 1.3, 0)
    scene.add(mesh)
    return { mesh, spin: 0.06 + (index % 3) * 0.03, baseY: y, phase: index }
  })

  const white = new THREE.Color(0xffffff)

  function setAccent(hex: string) {
    deep.color.set(hex)
    pale.color.set(hex).lerp(white, 0.4)
    rim.color.set(hex)
  }

  function setTheme(isDark: boolean) {
    background.set(backgroundColor(isDark))
    scene.environmentIntensity = isDark ? 0.5 : 1
    key.intensity = isDark ? 1.2 : 1.8
    rim.intensity = isDark ? 1.6 : 0.8
  }

  function render(time: number) {
    for (const crystal of crystals) {
      crystal.mesh.rotation.x = crystal.phase * 0.7 + time * crystal.spin
      crystal.mesh.rotation.y = crystal.phase * 1.3 + time * crystal.spin * 1.3
      crystal.mesh.position.y =
        crystal.baseY + Math.sin(time * 0.25 + crystal.phase) * 0.18
    }
    renderer.render(scene, camera)
  }

  setAccent(accent)
  setTheme(dark)

  return {
    setAccent,
    setTheme,
    resize: (w, h) => resizePerspective(camera, w, h),
    render,
    dispose() {
      disposeObject(scene)
      environment.dispose()
    },
  }
}

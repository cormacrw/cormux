import * as THREE from 'three'
import type { CreateScene } from '../types'
import {
  backgroundColor,
  disposeObject,
  perspective,
  resizePerspective,
  studioEnvironment,
} from './shared'

type Orb = {
  mesh: THREE.Mesh
  baseY: number
  speed: number
  phase: number
}

// x, y, z, radius, kind (0 glossy accent, 1 glass, 2 pale accent)
const LAYOUT: [number, number, number, number, 0 | 1 | 2][] = [
  [-3.6, 0.9, -1.5, 1.15, 0],
  [3.2, -0.6, -0.5, 1.4, 0],
  [0.6, 1.7, -3.5, 0.9, 2],
  [-1.3, -1.4, 0.6, 0.7, 1],
  [1.6, 0.9, 1.2, 0.5, 1],
  [-4.8, -1.8, -3.2, 0.8, 2],
  [5.2, 2.0, -4, 1.0, 1],
  [-0.4, 0.1, -1.4, 0.38, 0],
]

/** Glossy and glass spheres floating slowly in a soft void. */
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

  const key = new THREE.DirectionalLight(0xffffff, 1.2)
  key.position.set(-3, 5, 6)
  const glow = new THREE.PointLight(accent, 20, 16, 2)
  glow.position.set(0, -2, 2)
  scene.add(key, glow)

  const glossy = new THREE.MeshPhysicalMaterial({
    color: accent,
    roughness: 0.15,
    clearcoat: 1,
    clearcoatRoughness: 0.05,
  })
  const pale = new THREE.MeshPhysicalMaterial({
    color: accent,
    roughness: 0.4,
    clearcoat: 0.6,
    sheen: 1,
  })
  const glass = new THREE.MeshPhysicalMaterial({
    color: 0xffffff,
    roughness: 0.03,
    transparent: true,
    opacity: 0.25,
    clearcoat: 1,
    ior: 1.45,
    envMapIntensity: 1.6,
    depthWrite: false,
  })
  const materials = [glossy, glass, pale]
  const sphere = new THREE.SphereGeometry(1, 48, 32)
  const orbs: Orb[] = LAYOUT.map(([x, y, z, r, kind], index) => {
    const mesh = new THREE.Mesh(sphere, materials[kind])
    mesh.position.set(x, y, z)
    mesh.scale.setScalar(r)
    if (kind === 1) mesh.renderOrder = 2
    scene.add(mesh)
    return {
      mesh,
      baseY: y,
      speed: 0.18 + (index % 4) * 0.05,
      phase: index * 1.7,
    }
  })

  const paleColor = new THREE.Color()
  const white = new THREE.Color(0xffffff)

  function setAccent(hex: string) {
    glossy.color.set(hex)
    paleColor.set(hex).lerp(white, 0.45)
    pale.color.copy(paleColor)
    glass.color.set(hex).lerp(white, 0.7)
    glow.color.set(hex)
  }

  function setTheme(isDark: boolean) {
    background.set(backgroundColor(isDark))
    scene.environmentIntensity = isDark ? 0.5 : 1
    key.intensity = isDark ? 1 : 1.6
    glow.intensity = isDark ? 26 : 8
  }

  function render(time: number) {
    for (const orb of orbs) {
      orb.mesh.position.y =
        orb.baseY + Math.sin(time * orb.speed + orb.phase) * 0.3
    }
    camera.position.x = Math.sin(time * 0.04) * 0.6
    camera.lookAt(scene.position)
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

import * as THREE from 'three'
import { FontLoader } from 'three/examples/jsm/loaders/FontLoader.js'
import { TextGeometry } from 'three/examples/jsm/geometries/TextGeometry.js'
// Helvetiker Bold from three's examples, trimmed to the glyphs of "cormux"
// (the npm package no longer ships the fonts). Licence beside it.
import fontData from '../fonts/helvetiker-bold-cormux.typeface.json'
import type { CreateScene } from '../types'
import {
  backgroundColor,
  disposeObject,
  perspective,
  radialTexture,
  resizePerspective,
  studioEnvironment,
} from './shared'

/** The word "cormux" in extruded type on a studio floor, with props around it. */
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
  const fog = new THREE.Fog(0x000000, 10, 22)
  scene.fog = fog
  const environment = studioEnvironment(renderer)
  scene.environment = environment

  const camera = perspective(32, width, height, 0.1, 60)
  const target = new THREE.Vector3(0, 0.55, 0)
  let distance = 9

  // Lights: a soft key from above left, a cool fill, and an accent rim.
  const hemi = new THREE.HemisphereLight(0xffffff, 0x222222, 0.6)
  const key = new THREE.DirectionalLight(0xffffff, 1.6)
  key.position.set(-4, 7, 5)
  const rim = new THREE.PointLight(accent, 30, 14, 2)
  rim.position.set(2.5, 2.8, -3.5)
  scene.add(hemi, key, rim)

  // Type.
  const font = new FontLoader().parse(
    fontData as unknown as Parameters<FontLoader['parse']>[0],
  )
  const textGeometry = new TextGeometry('cormux', {
    font,
    size: 0.95,
    depth: 0.34,
    curveSegments: 10,
    bevelEnabled: true,
    bevelThickness: 0.035,
    bevelSize: 0.022,
    bevelSegments: 4,
  })
  textGeometry.computeBoundingBox()
  const box = textGeometry.boundingBox!
  textGeometry.translate(
    -(box.max.x + box.min.x) / 2,
    -box.min.y,
    -(box.max.z + box.min.z) / 2,
  )
  const textMaterial = new THREE.MeshPhysicalMaterial({
    color: accent,
    metalness: 0.15,
    roughness: 0.28,
    clearcoat: 1,
    clearcoatRoughness: 0.12,
  })
  const text = new THREE.Mesh(textGeometry, textMaterial)
  scene.add(text)

  // Props.
  const glossyMaterial = new THREE.MeshPhysicalMaterial({
    color: accent,
    roughness: 0.12,
    metalness: 0,
    clearcoat: 1,
    clearcoatRoughness: 0.04,
  })
  const glossy = new THREE.Mesh(
    new THREE.SphereGeometry(0.62, 48, 32),
    glossyMaterial,
  )
  glossy.position.set(-3.15, 0.62, 1.05)

  const chromeMaterial = new THREE.MeshStandardMaterial({
    color: 0xffffff,
    metalness: 1,
    roughness: 0.08,
  })
  const torus = new THREE.Mesh(
    new THREE.TorusGeometry(0.62, 0.17, 24, 72),
    chromeMaterial,
  )
  torus.position.set(2.85, 1.05, -1.1)
  torus.rotation.set(0.35, -0.6, 0.15)

  const glassMaterial = new THREE.MeshPhysicalMaterial({
    color: 0xffffff,
    metalness: 0,
    roughness: 0.02,
    transparent: true,
    opacity: 0.28,
    clearcoat: 1,
    clearcoatRoughness: 0,
    ior: 1.45,
    specularIntensity: 1,
    envMapIntensity: 1.6,
    depthWrite: false,
  })
  const glass = new THREE.Mesh(
    new THREE.SphereGeometry(0.34, 40, 28),
    glassMaterial,
  )
  glass.position.set(3.55, 0.34, 1.5)

  const capsule = new THREE.Mesh(
    new THREE.CapsuleGeometry(0.2, 0.75, 8, 24),
    chromeMaterial,
  )
  capsule.position.set(-1.7, 0.2, 1.85)
  capsule.rotation.set(0, -0.35, Math.PI / 2)
  scene.add(glossy, torus, glass, capsule)

  // Floor and soft contact shadows.
  const floorMaterial = new THREE.MeshStandardMaterial({
    roughness: 0.42,
    metalness: 0,
  })
  const floor = new THREE.Mesh(new THREE.PlaneGeometry(60, 60), floorMaterial)
  floor.rotation.x = -Math.PI / 2
  scene.add(floor)

  const shadowTexture = radialTexture()
  const shadowMaterial = new THREE.MeshBasicMaterial({
    map: shadowTexture,
    color: 0x000000,
    transparent: true,
    depthWrite: false,
  })
  const shadowGeometry = new THREE.PlaneGeometry(1, 1)
  const shadows: [THREE.Object3D, number, number][] = [
    [text, 6.4, 1.6],
    [glossy, 1.9, 1.9],
    [torus, 1.7, 1.2],
    [glass, 1.0, 1.0],
    [capsule, 1.6, 0.8],
  ]
  for (const [object, sx, sz] of shadows) {
    const shadow = new THREE.Mesh(shadowGeometry, shadowMaterial)
    shadow.rotation.x = -Math.PI / 2
    shadow.position.set(object.position.x, 0.003, object.position.z)
    shadow.scale.set(sx, sz, 1)
    shadow.renderOrder = 1
    scene.add(shadow)
  }

  const floorColor = new THREE.Color()

  function setAccent(hex: string) {
    textMaterial.color.set(hex)
    glossyMaterial.color.set(hex)
    rim.color.set(hex)
  }

  function setTheme(isDark: boolean) {
    background.set(backgroundColor(isDark))
    fog.color.copy(background)
    floorColor.set(isDark ? '#121418' : '#e6e9ed')
    floorMaterial.color.copy(floorColor)
    scene.environmentIntensity = isDark ? 0.55 : 0.95
    hemi.intensity = isDark ? 0.35 : 0.9
    key.intensity = isDark ? 1.3 : 1.9
    rim.intensity = isDark ? 18 : 10
    shadowMaterial.opacity = isDark ? 0.6 : 0.2
  }

  function resize(w: number, h: number) {
    resizePerspective(camera, w, h)
    // Pull back on narrow windows so the word still fits.
    distance = 9 / Math.min(1, camera.aspect / 1.55)
  }

  function render(time: number) {
    const drift = time * 0.045
    camera.position.set(
      Math.sin(drift) * 1.4,
      2.1 + Math.sin(drift * 0.7) * 0.25,
      distance + Math.cos(drift) * 0.4,
    )
    camera.lookAt(target)
    glossy.position.y = 0.62 + Math.sin(time * 0.5) * 0.04
    glass.position.y = 0.34 + Math.sin(time * 0.6 + 1) * 0.035
    torus.rotation.y = -0.6 + time * 0.12
    renderer.render(scene, camera)
  }

  setAccent(accent)
  setTheme(dark)
  resize(width, height)

  return {
    setAccent,
    setTheme,
    resize,
    render,
    dispose() {
      disposeObject(scene)
      shadowTexture.dispose()
      environment.dispose()
    },
  }
}

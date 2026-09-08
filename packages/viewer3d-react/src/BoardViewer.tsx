/**
 * BoardViewer — renders a Fiducial board GLB file in a Three.js WebGL canvas.
 *
 * Requires `three` ≥ 0.160 as a peer dependency.
 *
 * Usage:
 *   <BoardViewer src="/board.glb" width={600} height={400} />
 */

import { useEffect, useRef } from 'react'
import type { FC, CSSProperties } from 'react'
import * as THREE from 'three'
import { GLTFLoader } from 'three/examples/jsm/loaders/GLTFLoader.js'
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js'

export interface BoardViewerProps {
  /** URL of the `.glb` file to render. */
  src: string
  /** Canvas width in pixels (default: 600). */
  width?: number
  /** Canvas height in pixels (default: 400). */
  height?: number
  /** Background color (default: #1a1a2e). */
  background?: string
  /** CSS class applied to the container div. */
  className?: string
  /** Inline style applied to the container div. */
  style?: CSSProperties
}

/**
 * Three.js-based GLB viewer for Fiducial board meshes.
 *
 * Renders the GLB in a WebGL canvas with automatic camera framing and
 * mouse-driven orbit controls. The scene is torn down when the component
 * unmounts.
 */
export const BoardViewer: FC<BoardViewerProps> = ({
  src,
  width = 600,
  height = 400,
  background = '#1a1a2e',
  className,
  style,
}) => {
  const mountRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    const mount = mountRef.current
    if (!mount) return

    // Renderer
    const renderer = new THREE.WebGLRenderer({ antialias: true })
    renderer.setSize(width, height)
    renderer.setPixelRatio(globalThis.devicePixelRatio ?? 1)
    renderer.outputColorSpace = THREE.SRGBColorSpace
    mount.appendChild(renderer.domElement)

    // Scene
    const scene = new THREE.Scene()
    scene.background = new THREE.Color(background)

    // Camera
    const camera = new THREE.PerspectiveCamera(45, width / height, 0.01, 10000)
    camera.position.set(150, 80, 150)

    // Lights
    scene.add(new THREE.AmbientLight(0xffffff, 0.6))
    const dir = new THREE.DirectionalLight(0xffffff, 1.2)
    dir.position.set(100, 200, 100)
    scene.add(dir)

    // Orbit controls
    const controls = new OrbitControls(camera, renderer.domElement)
    controls.enableDamping = true

    // Load GLB
    const loader = new GLTFLoader()
    let frameId = 0
    loader.load(src, (gltf) => {
      const model = gltf.scene
      // Centre and fit the model
      const box = new THREE.Box3().setFromObject(model)
      const centre = new THREE.Vector3()
      box.getCenter(centre)
      const size = new THREE.Vector3()
      box.getSize(size)
      model.position.sub(centre)
      const maxDim = Math.max(size.x, size.y, size.z)
      camera.position.setLength(maxDim * 2.5)
      controls.target.copy(new THREE.Vector3())
      controls.update()
      scene.add(model)
    })

    // Render loop
    const animate = () => {
      frameId = requestAnimationFrame(animate)
      controls.update()
      renderer.render(scene, camera)
    }
    animate()

    return () => {
      cancelAnimationFrame(frameId)
      controls.dispose()
      renderer.dispose()
      mount.removeChild(renderer.domElement)
    }
  }, [src, width, height, background])

  return <div ref={mountRef} className={className} style={{ width, height, ...style }} />
}

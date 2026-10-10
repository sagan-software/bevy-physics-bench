import * as THREE from 'three';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
import type { Dimension, Snapshot } from './types';

/** Render engine snapshots. Drawing never advances a simulation. */
export class PhysicsView {
  readonly canvas = document.createElement('canvas');
  private renderer: THREE.WebGLRenderer;
  private scene = new THREE.Scene();
  private camera = new THREE.OrthographicCamera(-10, 10, 10, -10, .01, 100000);
  private controls: OrbitControls;
  private groups: { mesh: THREE.InstancedMesh; indices: number[] }[] = [];
  private links?: THREE.LineSegments;
  private extent = 12;
  private observer: ResizeObserver;
  private color: THREE.Color;
  private dummy = new THREE.Object3D();
  private grid: THREE.GridHelper;
  private ground: THREE.Mesh;
  onCameraChange?: () => void;
  onKick?: (index: number, velocity: [number, number, number]) => void;
  private snapshot?: Snapshot;
  private dimension: Dimension = '3d';

  constructor(private mount: HTMLElement, color: string, label: string) {
    this.color = new THREE.Color(color);
    this.canvas.setAttribute('aria-label', label);
    this.canvas.setAttribute('role', 'img');
    this.canvas.tabIndex = 0;
    this.renderer = new THREE.WebGLRenderer({ canvas: this.canvas, antialias: true, powerPreference: 'high-performance' });
    this.renderer.setPixelRatio(Math.min(devicePixelRatio, 2));
    this.renderer.setClearColor('#14212b');
    this.scene.add(new THREE.HemisphereLight(0xffffff, 0x334957, 2.4));
    const light = new THREE.DirectionalLight(0xffffff, 3.2);
    light.position.set(-10, 18, 12); this.scene.add(light);
    this.grid = new THREE.GridHelper(200, 100, 0x506271, 0x2b3d49); this.scene.add(this.grid);
    this.ground = new THREE.Mesh(new THREE.PlaneGeometry(2, 2), new THREE.MeshStandardMaterial({ color: '#192934', roughness: 1 }));
    this.ground.rotation.x = -Math.PI / 2; this.ground.position.y = -.02; this.scene.add(this.ground);
    this.controls = new OrbitControls(this.camera, this.canvas);
    this.controls.enableDamping = false;
    this.controls.addEventListener('change', () => { this.draw(); this.onCameraChange?.(); });
    this.mount.append(this.canvas);
    let pointer: { x: number; y: number; id: number; dragged: boolean } | undefined;
    this.canvas.addEventListener('pointerdown', event => { pointer = event.button === 0 && event.isPrimary ? { x: event.clientX, y: event.clientY, id: event.pointerId, dragged: false } : undefined; });
    this.canvas.addEventListener('pointermove', event => { if (pointer?.id === event.pointerId && !pointer.dragged && Math.hypot(event.clientX - pointer.x, event.clientY - pointer.y) > 5) pointer.dragged = true; });
    this.canvas.addEventListener('pointercancel', () => { pointer = undefined; });
    this.canvas.addEventListener('pointerup', event => {
      if (pointer?.id === event.pointerId && !pointer.dragged && Math.hypot(event.clientX - pointer.x, event.clientY - pointer.y) <= 5) this.pick(event.clientX, event.clientY);
      pointer = undefined;
    });
    this.canvas.addEventListener('keydown', event => {
      if (event.key === 'Enter' || event.key === ' ') {
        event.preventDefault(); const box = this.canvas.getBoundingClientRect(); this.pick(box.x + box.width / 2, box.y + box.height / 2);
      }
    });
    this.observer = new ResizeObserver(() => this.resize()); this.observer.observe(this.mount);
  }

  reset(snapshot: Snapshot, dimension: Dimension, color: string, floor: [number, number, number], fitCamera = true): void {
    this.dimension = dimension;
    this.ground.scale.set(floor[0], floor[2], 1);
    this.clearBodies(); this.color.set(color);
    const grouped = new Map<string, number[]>();
    snapshot.bodies.forEach((body, index) => {
      const key = JSON.stringify(body.shape);
      const group = grouped.get(key) ?? []; group.push(index); grouped.set(key, group);
    });
    for (const indices of grouped.values()) {
      const shape = snapshot.bodies[indices[0]].shape;
      const geometry = shape.kind === 'box'
        ? new THREE.BoxGeometry(...shape.half_extents.map(value => value * 2) as [number, number, number])
        : new THREE.SphereGeometry(shape.radius, 16, 12);
      const material = new THREE.MeshStandardMaterial({ color: 'white', roughness: .65, metalness: .05 });
      const mesh = new THREE.InstancedMesh(geometry, material, indices.length);
      mesh.instanceMatrix.setUsage(THREE.DynamicDrawUsage); mesh.frustumCulled = false;
      this.groups.push({ mesh, indices }); this.scene.add(mesh);
    }
    const geometry = new THREE.BufferGeometry();
    geometry.setAttribute('position', new THREE.BufferAttribute(new Float32Array(snapshot.joints.length * 6), 3));
    this.links = new THREE.LineSegments(geometry, new THREE.LineBasicMaterial({ color: 0xe8edf0 }));
    this.links.frustumCulled = false; this.scene.add(this.links);
    this.controls.enableRotate = dimension === '3d';
    this.grid.rotation.x = dimension === '2d' ? Math.PI / 2 : 0;
    this.grid.position.z = dimension === '2d' ? -.7 : 0;
    this.ground.visible = dimension === '3d';
    if (fitCamera) this.fit(snapshot, dimension);
    this.update(snapshot);
  }

  update(snapshot: Snapshot): void {
    this.snapshot = snapshot;
    const muted = new THREE.Color('#576c78'), active = this.color.clone(), white = new THREE.Color('white');
    for (const { mesh, indices } of this.groups) {
      indices.forEach((bodyIndex, instanceIndex) => {
        const body = snapshot.bodies[bodyIndex];
        this.dummy.position.fromArray(body.position); this.dummy.quaternion.fromArray(body.rotation);
        this.dummy.updateMatrix(); mesh.setMatrixAt(instanceIndex, this.dummy.matrix);
        const color = active.copy(body.kind === 'static' ? muted : this.color);
        if (body.sleeping) color.lerp(muted, .6);
        if (body.kind === 'kinematic') color.lerp(white, .45);
        mesh.setColorAt(instanceIndex, color);
      });
      mesh.instanceMatrix.needsUpdate = true;
      if (mesh.instanceColor) mesh.instanceColor.needsUpdate = true;
    }
    if (this.links) {
      const positions = this.links.geometry.getAttribute('position') as THREE.BufferAttribute;
      snapshot.joints.forEach(([first, second], index) => {
        positions.setXYZ(index * 2, ...snapshot.bodies[first].position);
        positions.setXYZ(index * 2 + 1, ...snapshot.bodies[second].position);
      });
      positions.needsUpdate = true;
    }
    this.draw();
  }

  /** Pick one visible dynamic body; both engines receive its stable scene index. */
  private pick(x: number, y: number): void {
    const box = this.canvas.getBoundingClientRect();
    const ray = new THREE.Raycaster();
    ray.setFromCamera(new THREE.Vector2((x - box.x) / box.width * 2 - 1, 1 - (y - box.y) / box.height * 2), this.camera);
    for (const { mesh } of this.groups) mesh.computeBoundingSphere();
    for (const hit of ray.intersectObjects(this.groups.map(group => group.mesh))) {
      const group = this.groups.find(group => group.mesh === hit.object)!;
      const index = group.indices[hit.instanceId!];
      if (this.snapshot?.bodies[index].kind !== 'dynamic') continue;
      const direction = this.dimension === '2d' ? new THREE.Vector3(hit.point.x >= this.controls.target.x ? 1 : -1, 0, 0) : new THREE.Vector3(ray.ray.direction.x, 0, ray.ray.direction.z).normalize();
      this.onKick?.(index, [direction.x * 12, 6, direction.z * 12]); break;
    }
  }

  syncCamera(other: PhysicsView): void {
    this.camera.position.copy(other.camera.position); this.camera.quaternion.copy(other.camera.quaternion);
    this.camera.zoom = other.camera.zoom; this.controls.target.copy(other.controls.target);
    this.camera.updateProjectionMatrix(); this.draw();
  }

  private fit(snapshot: Snapshot, dimension: Dimension): void {
    const box = new THREE.Box3();
    for (const body of snapshot.bodies) box.expandByPoint(new THREE.Vector3(...body.position));
    box.min.y = Math.min(0, box.min.y); box.expandByScalar(1);
    const centre = box.getCenter(new THREE.Vector3()); const size = box.getSize(new THREE.Vector3());
    const aspect = Math.max(.3, this.mount.clientWidth / Math.max(1, this.mount.clientHeight));
    this.extent = Math.max(6, size.y * .7, size.x / aspect * .75, size.z * .7);
    this.camera.zoom = 1;
    const offset = dimension === '2d' ? new THREE.Vector3(0, 0, 100) : new THREE.Vector3(60, 45, 70);
    this.camera.position.copy(centre).add(offset); this.controls.target.copy(centre);
    this.camera.lookAt(centre); this.controls.update(); this.resize();
  }

  private resize(): void {
    const width = Math.max(1, this.mount.clientWidth), height = Math.max(1, this.mount.clientHeight);
    this.renderer.setSize(width, height, false);
    this.camera.left = -this.extent * width / height; this.camera.right = -this.camera.left;
    this.camera.top = this.extent; this.camera.bottom = -this.extent;
    this.camera.updateProjectionMatrix(); this.draw();
  }
  private draw(): void { this.renderer.render(this.scene, this.camera); }
  private clearBodies(): void {
    for (const { mesh } of this.groups) { this.scene.remove(mesh); mesh.geometry.dispose(); (mesh.material as THREE.Material).dispose(); }
    this.groups = [];
    if (this.links) { this.scene.remove(this.links); this.links.geometry.dispose(); (this.links.material as THREE.Material).dispose(); }
  }
  dispose(): void {
    this.clearBodies(); this.observer.disconnect(); this.controls.dispose();
    this.grid.geometry.dispose(); (this.grid.material as THREE.Material).dispose();
    this.ground.geometry.dispose(); (this.ground.material as THREE.Material).dispose();
    this.renderer.dispose(); this.canvas.remove();
  }
}

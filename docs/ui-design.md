# Newell CAD - UI Design & Rendering Specifications

**Target:** `wgpu` ASCII-grid based UI
**Status:** Active

This document acts as the single source of truth for all UI elements, layout behaviors, and rendering rules in Newell. The Architect and Executor MUST adhere to these specifications when planning or implementing the user interface.

## 1. Grid System & Typography
* **Font Family:** `Fira Code` (Monospaced).
* **Grid Cell Ratio:** `3:5` (Width:Height). All UI coordinate calculations must respect this physical aspect ratio to prevent distortion.
* **Color Support:** The text renderer must support multiple font colors and dynamic background colors per cell.
* **Section Separators:** UI panels and logical sections are strictly delineated using double-line box-drawing characters: `╔`, `╗`, `╚`, `╝`, `║`, `═`.
  * *Example:* 
    `╔════════════════╗`
    `║   PROPERTIES   ║`
    `╚════════════════╝`

## 2. Iconography & Atlas System
Logos and icons are rendered as textured quads overlaid on the ASCII grid. 
* **Texture Atlas:** All icons must be packed into a single `.png` texture atlas. 
* **Atlas Dimensions:** The atlas dimensions must be a power of two (e.g., $1024 \times 1024$, $2048 \times 2048$ pixels) to comply with optimal GPU memory mapping.
* **Icon Resolution:** Each individual icon within the atlas is drawn at $256 \times 256$ pixels. This high resolution allows crisp downscaling across various monitor DPIs.
* **Padding & Bleed Prevention:** Every $256 \times 256$ icon requires an 8-pixel internal transparent padding. The actual drawn art must be confined to the central $240 \times 240$ pixels. This prevents texture bleeding and artifacts when the GPU applies mipmapping during downscaling.
* **Color & Transparency:** Icons fully support multiple colors and alpha-channel transparency. They are not restricted to monochrome.
* **Grid Alignment:** An icon occupies exactly 4 cells arranged in a $2 \times 2$ block. The icon texture must be perfectly centered over these 4 cells.

## 3. Interactivity & State Changes
* **Hover State:** When the mouse pointer hovers over an interactive cell (button, menu, tree item), the background color tone of that specific cell (or group of cells) must change to provide immediate visual feedback.
* **Hitboxes:** Icon hitboxes span their underlying $2 \times 2$ cell cluster.

## 4. Input Controls & Menus
Controls adapt based on the number of available choices to maintain a clean 2D aesthetic.

* **Small Selectors ($\le 3$ options):** 
  Rendered inline directly within the side panels using left/right indicator arrows.
  * *Format:* `◄ OptionName ▶`
  * *Behavior:* Clicking the arrows cycles through the available options inline.

* **Large Menus ($> 3$ options) & Top Bar Menus:**
  Used for Top-Bar navigation (`File`, `Edit`, `View`) or complex dropdowns.
  * *Behavior:* Clicking opens a 2D modal window perfectly centered in the main 3D viewport.
  * *Z-Indexing:* The modal pauses interactions with the 3D viewport behind it and renders an opaque background (potentially with a subtle ASCII pattern or darkened tone) to ensure readability.

## 5. Feature Tree (Arbre CAO)
The parametric history and assembly structure is displayed using a strict ASCII-line hierarchy. 
* **Collapse/Expand:** Folders and parametric nodes use `[-]` to collapse children and `[+]` to expand them.
* **Formatting Rules:** Use standard single-line drawing characters (`├─`, `└─`, `│`) to build the tree structure.
* **Reference Example:**
  ```text
  [-] ARBRE CAO
   ├─ [+] Origine
   ├─ [-] Assemblage_Roue
   │   ├─ [+] Axe_Central
   │   ├─ [-] Jante
   │   │   ├─ [+] Esquisse
   │   │   └─ [+] Revolution
   │   └─ [+] Pneu
   └─ [+] Vis_M8
  
## 6. Rendering PipelineConstraints
Pass 1: Render the base ASCII background colors.
Pass 2: Render the text characters (Fira Code glyphs).
Pass 3: Render the $2 \times 2$ atlas icons over the grid, respecting alpha blending.
Pass 4: Render Viewport Modals (if active) on the highest Z-layer, capturing all mouse/keyboard events until closed.

## 7. Global Layout Structure (6-Pane Layout)
The main application window is strictly divided into 6 distinct logical areas overlaid on the ASCII grid. 

* **Top:** `Menu Bar` (File, Edit, View, etc.)
* **Below Menu:** `Toolbar` (Quick actions, tool icons from the atlas)
* **Left:** `Tree` (Arbre CAO / Feature Tree)
* **Right:** `Task` (Properties, context-specific tool settings)
* **Bottom:** `State Info` (Status bar, coordinates, FPS, hints)
* **Center:** `3D Viewport` (The actual CAD rendering area)

The boundaries between these areas are drawn using the double-line box-drawing characters (e.g., `║`, `═`, `╬`).

## 8. Responsiveness & Resizing Behavior
Since the UI is a Character Grid, all resizing operations—whether driven by the OS window or user interaction—must be strictly **discrete (character-by-character)**. There is no sub-cell resizing.

### 8.1 OS Window Resizing
When the user resizes the main OS window, the internal grid dynamically re-allocates its cells:
* **Menu Bar, Toolbar, State Info:** Expand/contract in **width** only (height in cells remains fixed).
* **Tree (Left) & Task (Right):** Expand/contract in **height** only (width in cells remains fixed unless dragged by the user).
* **3D Viewport (Center):** Expands/contracts in **both width and height** to perfectly fill the remaining space.

### 8.2 User-Driven Panel Resizing (Mouse Grab)
The user can customize the width of the side panels.
* **Interaction:** Hovering over the vertical borders (`║`) between the Tree/Viewport or Viewport/Task changes the cursor to a resize grabber. Clicking and dragging moves the border.
* **Grid Snapping:** The border visually snaps from column to column. It jumps one full cell width at a time.
* **Viewport Compensation:** As the Tree or Task panels increase or decrease in width, the central 3D Viewport automatically shrinks or expands to absorb the delta.

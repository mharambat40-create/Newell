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

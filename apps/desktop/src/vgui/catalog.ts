// Known VGUI/TF2 properties: editor type, group and a one-line explanation.
export type PropType = 'pos' | 'size' | 'int' | 'float' | 'bool' | 'color' | 'font' | 'text' | 'string' | 'image' | 'alignment' | 'pin' | 'corner' | 'border' | 'control';

export type PropInfo = { type: PropType; group: string; doc: string };

const P = (type: PropType, group: string, doc: string): PropInfo => ({ type, group, doc });

export const PROPS: Record<string, PropInfo> = {
  controlname: P('control', 'Identity', 'VGUI class to create (CExLabel, ImagePanel, EditablePanel…). Only used for elements the .res file adds.'),
  fieldname: P('string', 'Identity', "The panel's name. Animations, pins and game code find panels by this name."),
  xpos: P('pos', 'Layout', 'Horizontal position. 10 = from the left, r10 = from the right, c-10 = from the center, cs-0.5 = centered on itself, rs1 = right-aligned. With pin_to_sibling it is an offset.'),
  ypos: P('pos', 'Layout', 'Vertical position, same notation as xpos (r = from the bottom).'),
  zpos: P('int', 'Layout', 'Draw order among siblings; higher draws on top.'),
  wide: P('size', 'Layout', 'Width in units (480 units = screen height). f0 = full parent width, f20 = full minus 20, p0.5 = half the parent, o1 = equal to tall.'),
  tall: P('size', 'Layout', 'Height in units. Same notation as wide (o = relative to wide).'),
  visible: P('bool', 'Layout', 'Shown when 1. Game code and animations often toggle this.'),
  enabled: P('bool', 'Layout', 'Accepts input and draws in its enabled colors when 1.'),
  proportionaltoparent: P('bool', 'Layout', 'Scale this panel relative to its parent instead of the screen.'),
  pin_to_sibling: P('pin', 'Pinning', 'Attach to another panel in the same file; xpos/ypos become offsets away from the pinned corner.'),
  pin_corner_to_sibling: P('corner', 'Pinning', "This panel's corner that is attached."),
  pin_to_sibling_corner: P('corner', 'Pinning', "The sibling's corner it is attached to."),
  labeltext: P('text', 'Text', 'Text to show. #Token uses the localization files; %variable% is filled in by the game.'),
  font: P('font', 'Text', 'Scheme font (defined in clientscheme / fonts.res).'),
  font_minmode: P('font', 'Text', 'Font used with cl_hud_minmode 1.'),
  textalignment: P('alignment', 'Text', 'Where text sits inside the panel.'),
  textinsetx: P('int', 'Text', 'Horizontal text padding in units.'),
  textinsety: P('int', 'Text', 'Vertical text padding in units.'),
  wrap: P('bool', 'Text', 'Wrap long text onto several lines.'),
  centerwrap: P('bool', 'Text', 'Wrap and center each line.'),
  allcaps: P('bool', 'Text', 'Render text in capitals.'),
  use_proportional_insets: P('bool', 'Text', 'Scale text insets with the screen.'),
  fgcolor: P('color', 'Colors', 'Foreground (text/fill) color. Scheme name or "R G B A".'),
  fgcolor_override: P('color', 'Colors', 'Overrides the scheme-chosen foreground color.'),
  bgcolor_override: P('color', 'Colors', 'Background color (drawn when paintbackground is on).'),
  bgcolor: P('color', 'Colors', 'Background color.'),
  paintbackground: P('bool', 'Colors', 'Draw the background color.'),
  paintbackgroundtype: P('int', 'Colors', '0 = square, 2 = rounded corners.'),
  paintborder: P('bool', 'Colors', 'Draw the border.'),
  border: P('border', 'Colors', 'Scheme border name.'),
  alpha: P('int', 'Colors', 'Opacity 0–255.'),
  defaultfgcolor_override: P('color', 'Colors', 'Button text color. Unset: the scheme’s Button.TextColor.'),
  defaultbgcolor_override: P('color', 'Colors', 'Button background. Unset: the scheme’s Button.BgColor.'),
  armedfgcolor_override: P('color', 'Colors', 'Button text color under the mouse. Unset: Button.ArmedTextColor, else the default text color.'),
  armedbgcolor_override: P('color', 'Colors', 'Button background under the mouse. Unset: Button.ArmedBgColor, else the default background.'),
  depressedfgcolor_override: P('color', 'Colors', 'Button text color while pressed. Unset: the default text color.'),
  depressedbgcolor_override: P('color', 'Colors', 'Button background while pressed. Unset: the default background.'),
  image_drawcolor: P('color', 'Colors', 'Tint of an image button’s SubImage.'),
  image_armedcolor: P('color', 'Colors', 'SubImage tint under the mouse. Unset: image_drawcolor.'),
  border_default: P('border', 'Colors', 'Button border in its normal state.'),
  border_armed: P('border', 'Colors', 'Button border under the mouse.'),
  image: P('image', 'Image', 'Material under materials/vgui (no extension). ../ leaves the vgui folder.'),
  scaleimage: P('bool', 'Image', 'Stretch the image to the panel size.'),
  tileimage: P('bool', 'Image', 'Repeat the image across the panel.'),
  fillcolor: P('color', 'Image', 'Solid color drawn behind the image (or alone without an image).'),
  drawcolor: P('color', 'Image', 'Tint multiplied with the image.'),
  drawcolor_override: P('color', 'Image', 'Overrides the image tint.'),
  teambg_1: P('image', 'Image', 'Image used for spectators/unassigned (CTFImagePanel).'),
  teambg_2: P('image', 'Image', 'Image used on RED (CTFImagePanel).'),
  teambg_3: P('image', 'Image', 'Image used on BLU (CTFImagePanel).'),
  src_corner_height: P('int', 'Image', 'ScalableImagePanel: source corner height in texels.'),
  src_corner_width: P('int', 'Image', 'ScalableImagePanel: source corner width in texels.'),
  draw_corner_width: P('int', 'Image', 'ScalableImagePanel: drawn corner width in units.'),
  draw_corner_height: P('int', 'Image', 'ScalableImagePanel: drawn corner height in units.'),
  progress: P('float', 'Progress', 'Preview fill of progress bars (0–1).'),
  autoresize: P('int', 'Layout', 'Resize with parent: 0 none, 1 right, 2 down, 3 both.'),
  pinCorner: P('int', 'Layout', 'Legacy pin to the parent corner.'),
  tabposition: P('int', 'Input', 'Keyboard tab order.'),
  command: P('string', 'Input', 'Command sent when a button is pressed.'),
  mouseinputenabled: P('bool', 'Input', 'Receives mouse input.'),
  keyboardinputenabled: P('bool', 'Input', 'Receives keyboard input.'),
};

export const PROP_GROUPS = ['Identity', 'Layout', 'Pinning', 'Text', 'Colors', 'Image', 'Progress', 'Input', 'Other'];

export function propInfo(key: string): PropInfo {
  const k = key.toLowerCase();
  if (PROPS[k]) return PROPS[k];
  if (k.endsWith('color') || k.endsWith('color_override') || /color\d_override$/.test(k)) return P('color', 'Colors', 'Color (scheme name or "R G B A").');
  if (k.endsWith('font')) return P('font', 'Text', 'Scheme font.');
  if (k.startsWith('if_')) return P('string', 'Other', 'Conditional block.');
  return P('string', 'Other', '');
}

export type ControlTemplate = { label: string; control: string; props: [string, string][] };

export const CONTROL_TEMPLATES: ControlTemplate[] = [
  { label: 'Label', control: 'CExLabel', props: [['xpos', 'c-50'], ['ypos', 'c-10'], ['zpos', '5'], ['wide', '100'], ['tall', '20'], ['visible', '1'], ['enabled', '1'], ['labelText', 'New label'], ['textAlignment', 'center'], ['fgcolor', '255 255 255 255']] },
  { label: 'Image', control: 'ImagePanel', props: [['xpos', 'c-16'], ['ypos', 'c-16'], ['zpos', '2'], ['wide', '32'], ['tall', '32'], ['visible', '1'], ['enabled', '1'], ['image', 'replay/thumbnails/star'], ['scaleImage', '1']] },
  { label: 'Color box', control: 'ImagePanel', props: [['xpos', 'c-40'], ['ypos', 'c-20'], ['zpos', '1'], ['wide', '80'], ['tall', '40'], ['visible', '1'], ['enabled', '1'], ['fillcolor', '0 0 0 150']] },
  { label: 'Container', control: 'EditablePanel', props: [['xpos', 'c-75'], ['ypos', 'c-40'], ['zpos', '1'], ['wide', '150'], ['tall', '80'], ['visible', '1'], ['enabled', '1'], ['bgcolor_override', '0 0 0 100']] },
  { label: 'Progress bar', control: 'ContinuousProgressBar', props: [['xpos', 'c-50'], ['ypos', 'c30'], ['zpos', '3'], ['wide', '100'], ['tall', '6'], ['visible', '1'], ['enabled', '1'], ['fgcolor_override', '255 255 255 255'], ['bgcolor_override', '0 0 0 120']] },
  { label: 'Button', control: 'CExButton', props: [['xpos', 'c-50'], ['ypos', 'c-12'], ['zpos', '5'], ['wide', '100'], ['tall', '24'], ['visible', '1'], ['enabled', '1'], ['labelText', 'Button'], ['textAlignment', 'center'], ['command', ''], ['paintbackground', '1'], ['defaultBgColor_override', '40 40 40 255'], ['armedBgColor_override', '70 70 70 255']] },
];

/** Control kinds as the renderer understands them. */
export function controlKind(control: string): 'label' | 'image' | 'progress' | 'model' | 'button' | 'panel' | 'richtext' | 'avatar' {
  const c = control.toLowerCase();
  if (c.includes('progressbar') || c.includes('healthbar')) return 'progress';
  if (c.includes('modelpanel') || c === 'cembeddeditemmodelpanel') return 'model';
  if (c.includes('avatar')) return 'avatar';
  if (c.includes('richtext') || c === 'textentry') return 'richtext';
  if (c.includes('button') || c.includes('checkbutton') || c === 'ccvarslider' || c === 'combobox') return 'button';
  if (c.includes('label') || c.includes('variablelabel')) return 'label';
  if (c.includes('image') || c.includes('iconpanel') || c === 'ctfclassimage' || c === 'ctflogopanel') return 'image';
  return 'panel';
}

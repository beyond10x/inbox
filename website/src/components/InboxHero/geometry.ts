export interface Box {
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface Point {
  x: number;
  y: number;
}

export type Place = 'sender' | 'recipient' | 'inbox' | 'planned';
export type Agent = 'sender' | 'recipient' | 'planned';
export type LayoutName = 'wide' | 'tall';

/** Where everything of the figure sits, in the units of its viewBox. */
export interface Geometry {
  name: LayoutName;
  view: {w: number; h: number};
  hosts: {today: Box; planned: Box};
  processes: Record<Place, Box>;
  agent: Record<Agent, Point>;
  /** Each agent's line to the inbox, from the edge of its process to the edge of the inbox; a message travels along it. */
  links: Record<Agent, {from: Point; to: Point}>;
  /** Where a message rests while the inbox holds it: in the tray. */
  slot: Point;
  /** The column inside the inbox where the state, the views and the service's own operations are listed. */
  rows: {x: number; top: number; step: number};
  inboxTitle: Point;
  plannedLabel: Point;
  legend: Point;
  /** The operation on an agent's line, and the refusal under it. */
  callout: (agent: Agent) => {command: Point; refusal: Point};
  /** Glyphs of the direction of an operation: toward the inbox and toward the agent. */
  toward: {inbox: string; actor: string};
  /** A label is moved sideways to stay inside these x bounds. */
  bounds: {min: number; max: number};
}

/** Font size of every text of the figure, in viewBox units. */
export const FONT = 16;
const CHAR = FONT * 0.62;
export const PILL = {h: 28, pad: 12, glyph: 18};

/** Width of a pill in the monospace face of the figure. */
export function labelWidth(text: string, glyph = false): number {
  return Math.round(text.length * CHAR + 2 * PILL.pad + (glyph ? PILL.glyph : 0));
}

function centre(box: Box): Point {
  return {x: box.x + box.w / 2, y: box.y + box.h / 2};
}

export function midpoint(geometry: Geometry, agent: Agent): Point {
  const {from, to} = geometry.links[agent];
  return {x: (from.x + to.x) / 2, y: (from.y + to.y) / 2};
}

function wide(): Geometry {
  const processes: Record<Place, Box> = {
    sender: {x: 16, y: 60, w: 88, h: 96},
    recipient: {x: 16, y: 300, w: 88, h: 96},
    inbox: {x: 304, y: 46, w: 134, h: 370},
    planned: {x: 478, y: 172, w: 108, h: 96},
  };
  const agent = {sender: centre(processes.sender), recipient: centre(processes.recipient), planned: centre(processes.planned)};
  const links = {
    sender: {from: {x: 104, y: agent.sender.y}, to: {x: 304, y: agent.sender.y}},
    recipient: {from: {x: 104, y: agent.recipient.y}, to: {x: 304, y: agent.recipient.y}},
    planned: {from: {x: 478, y: agent.planned.y}, to: {x: 438, y: agent.planned.y}},
  };
  const geometry: Geometry = {
    name: 'wide',
    view: {w: 600, h: 462},
    hosts: {today: {x: 2, y: 30, w: 446, h: 400}, planned: {x: 466, y: 140, w: 132, h: 160}},
    processes,
    agent,
    links,
    slot: {x: 371, y: 130},
    rows: {x: 371, top: 200, step: 34},
    inboxTitle: {x: 371, y: 78},
    plannedLabel: {x: 532, y: 330},
    legend: {x: 16, y: 456},
    callout: (who) => {
      const at = midpoint(geometry, who);
      return {command: {x: at.x, y: at.y - 25}, refusal: {x: at.x, y: at.y + 25}};
    },
    toward: {inbox: '→', actor: '←'},
    bounds: {min: 0, max: 600},
  };
  return geometry;
}

function tall(): Geometry {
  const processes: Record<Place, Box> = {
    sender: {x: 20, y: 50, w: 96, h: 92},
    recipient: {x: 244, y: 50, w: 96, h: 92},
    inbox: {x: 20, y: 262, w: 320, h: 190},
    planned: {x: 132, y: 514, w: 96, h: 92},
  };
  const agent = {sender: centre(processes.sender), recipient: centre(processes.recipient), planned: centre(processes.planned)};
  const links = {
    sender: {from: {x: agent.sender.x, y: 142}, to: {x: agent.sender.x, y: 262}},
    recipient: {from: {x: agent.recipient.x, y: 142}, to: {x: agent.recipient.x, y: 262}},
    planned: {from: {x: agent.planned.x, y: 514}, to: {x: agent.planned.x, y: 452}},
  };
  const geometry: Geometry = {
    name: 'tall',
    view: {w: 360, h: 652},
    hosts: {today: {x: 2, y: 30, w: 356, h: 436}, planned: {x: 20, y: 500, w: 320, h: 120}},
    processes,
    agent,
    links,
    slot: {x: 84, y: 360},
    rows: {x: 244, top: 296, step: 32},
    inboxTitle: {x: 84, y: 300},
    plannedLabel: {x: 286, y: 560},
    legend: {x: 20, y: 646},
    callout: (who) => {
      const at = midpoint(geometry, who);
      return {command: {x: at.x, y: at.y - 21}, refusal: {x: at.x, y: at.y + 21}};
    },
    toward: {inbox: '↓', actor: '↑'},
    bounds: {min: 8, max: 352},
  };
  return geometry;
}

export const LAYOUTS: Record<LayoutName, Geometry> = {wide: wide(), tall: tall()};

/** Below this rendered width of the figure the tall layout is drawn, so its labels keep their size. */
export const TALL_BELOW_PX = 480;

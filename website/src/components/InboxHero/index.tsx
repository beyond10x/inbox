import {createContext, useContext, useEffect, useRef, useState, type CSSProperties, type ReactNode, type RefObject} from 'react';
import animation from '@site/src/generated/animation.json';
import {LAYOUTS, PILL, TALL_BELOW_PX, labelWidth, midpoint, type Agent, type Box, type Geometry, type LayoutName, type Point} from './geometry';
import './styles.css';

type Actor = 'sender' | 'recipient' | 'service';

interface StillStep {
  actor: Actor;
  command: string;
  toward: 'inbox' | 'actor';
}

export interface AnimationFrame {
  actor: Actor;
  command: string;
  journey: number;
  refusal: string | null;
  removed: boolean;
  state: string | null;
  step: number;
  token: 'inbox' | 'sender' | 'recipient' | null;
  views: string[];
  visible: string[];
  words: number;
}

// The data file is regenerated while the dev server runs; a field it lacks falls back here instead of failing the page.
const data = animation as Partial<typeof animation>;
const LAYOUT = {host: 'host', process: 'process', service: 'inbox', planned: 'planned', ...(data.layout ?? {})};
const STILL: StillStep[] = Array.isArray(data.still) ? (data.still as StillStep[]) : [];
export const FRAMES: AnimationFrame[] = Array.isArray(data.frames)
  ? (data.frames as AnimationFrame[]).map((frame) => ({...frame, views: Array.isArray(frame.views) ? frame.views : []}))
  : [];
const FRAME_MS = typeof data.frame_ms === 'number' && data.frame_ms > 0 ? data.frame_ms : 2600;

const GeometryContext = createContext<Geometry>(LAYOUTS.wide);
const useGeometry = () => useContext(GeometryContext);

function Frame({box, className, label}: {box: Box; className: string; label?: ReactNode}) {
  return (
    <g className={className}>
      <rect x={box.x} y={box.y} width={box.w} height={box.h} rx={12} />
      {label}
    </g>
  );
}

function AgentGlyph({at, className = ''}: {at: Point; className?: string}) {
  return (
    <g className={`inbox-hero__agent ${className}`} transform={`translate(${at.x} ${at.y + 6})`} aria-hidden="true">
      <circle r={24} className="inbox-hero__head" />
      <rect x={-12} y={-8} width={24} height={16} rx={4} className="inbox-hero__face" />
      <circle cx={-5.5} cy={0} r={2.4} className="inbox-hero__eye" />
      <circle cx={5.5} cy={0} r={2.4} className="inbox-hero__eye" />
      <line x1={0} y1={-24} x2={0} y2={-32} className="inbox-hero__antenna" />
      <circle cx={0} cy={-34} r={2.8} className="inbox-hero__eye" />
    </g>
  );
}

function Link({agent, className = ''}: {agent: Agent; className?: string}) {
  const {from, to} = useGeometry().links[agent];
  return <line className={`inbox-hero__link ${className}`} x1={from.x} y1={from.y} x2={to.x} y2={to.y} />;
}

/** A pill centred on a point and kept inside the figure; the glyph, when given, is drawn and not counted as a word. */
function Label({at, text, tone, glyph}: {at: Point; text: string; tone: string; glyph?: string}) {
  const {bounds} = useGeometry();
  const width = labelWidth(text, Boolean(glyph));
  const left = Math.min(Math.max(at.x - width / 2, bounds.min), bounds.max - width);
  return (
    <g className={`inbox-hero__label inbox-hero__label--${tone}`} transform={`translate(${left} ${at.y})`}>
      <rect x={0} y={-PILL.h / 2} width={width} height={PILL.h} rx={PILL.h / 2} />
      {glyph && (
        <text x={PILL.pad} y={5.5} aria-hidden="true" className="inbox-hero__glyph">
          {glyph}
        </text>
      )}
      <text x={PILL.pad + (glyph ? PILL.glyph : 0)} y={5.5} data-word="">
        {text}
      </text>
    </g>
  );
}

function Envelope({at, style, className = ''}: {at?: Point; style?: CSSProperties; className?: string}) {
  return (
    <g
      className={`inbox-hero__token ${className}`}
      style={style}
      transform={at ? `translate(${at.x} ${at.y})` : undefined}
      aria-hidden="true">
      <rect x={-16} y={-11} width={32} height={22} rx={3} />
      <path d="M -15 -9 L 0 3 L 15 -9" />
    </g>
  );
}

/** The still: one message on the sender's line, with the operation that carries it. */
function Still() {
  const geometry = useGeometry();
  const step = STILL.find((s) => s.actor === 'sender' && s.toward === 'inbox') ?? STILL[0];
  if (!step) return null;
  const agent = step.actor === 'recipient' ? 'recipient' : 'sender';
  return (
    <g className="inbox-hero__still" data-testid="hero-still">
      <Link agent={agent} className="inbox-hero__link--active" />
      <Label at={geometry.callout(agent).command} text={step.command} tone="to-inbox" glyph={geometry.toward[step.toward]} />
      <Envelope at={midpoint(geometry, agent)} />
    </g>
  );
}

function place(geometry: Geometry, token: AnimationFrame['token'] | Actor): Point {
  // At an agent the message rests on the edge of its process, so the agent stays visible.
  if (token === 'sender' || token === 'recipient') return geometry.links[token].from;
  return geometry.slot;
}

/** The path of the message in a frame: from where the previous frame of its journey left it, along an agent's line. */
function route(geometry: Geometry, frame: AnimationFrame, previous: AnimationFrame | undefined): Point[] {
  const to = place(geometry, frame.token);
  let from: Point;
  if (previous && previous.step === frame.step) from = to;
  else if (previous && previous.journey === frame.journey && previous.token && !previous.removed) from = place(geometry, previous.token);
  else from = place(geometry, frame.actor);
  const agent = frame.token === 'sender' || frame.token === 'recipient' ? frame.token : frame.actor;
  if (agent === 'service' || (from.x === to.x && from.y === to.y)) return [from, from, to];
  return [from, geometry.links[agent].to, to];
}

function InboxColumn({frame}: {frame: AnimationFrame}) {
  const {rows} = useGeometry();
  const items: {text: string; tone: string; glyph?: string}[] = [];
  if (frame.actor === 'service') {
    items.push({text: frame.command, tone: 'service'});
    if (frame.refusal) items.push({text: frame.refusal, tone: 'refusal', glyph: '✕'});
  }
  if (frame.state) items.push({text: frame.state, tone: 'state'});
  return (
    <g data-testid="hero-inbox-column">
      {items.map((item, i) => (
        <Label key={`${item.tone}-${item.text}`} at={{x: rows.x, y: rows.top + i * rows.step}} {...item} />
      ))}
      <g data-testid="hero-views">
        {frame.views.map((view, i) => (
          <Label key={view} at={{x: rows.x, y: rows.top + (items.length + i) * rows.step + 8}} text={view} tone="view" />
        ))}
      </g>
    </g>
  );
}

function Moving({frame, previous}: {frame: AnimationFrame; previous: AnimationFrame | undefined}) {
  const geometry = useGeometry();
  const key = `${frame.step}-${frame.refusal ?? ''}`;
  const actor = frame.actor;
  const toward = frame.token === 'inbox' || frame.token === null ? 'inbox' : 'actor';
  const [from, via, to] = route(geometry, frame, previous);
  const style = {
    '--from-x': `${from.x}px`,
    '--from-y': `${from.y}px`,
    '--via-x': `${via.x}px`,
    '--via-y': `${via.y}px`,
    '--to-x': `${to.x}px`,
    '--to-y': `${to.y}px`,
  } as CSSProperties;
  const inbox = geometry.processes.inbox;
  return (
    <g className="inbox-hero__moving" data-testid="hero-frame" data-command={frame.command} data-words={frame.words}>
      {actor !== 'service' ? (
        <Link agent={actor} className="inbox-hero__link--active" />
      ) : (
        <rect className="inbox-hero__pulse" x={inbox.x - 5} y={inbox.y - 5} width={inbox.w + 10} height={inbox.h + 10} rx={16} />
      )}
      <g key={`labels-${key}`} className="inbox-hero__appear">
        {actor !== 'service' && (
          <>
            <Label
              at={geometry.callout(actor).command}
              text={frame.command}
              tone={toward === 'inbox' ? 'to-inbox' : 'to-agent'}
              glyph={geometry.toward[toward]}
            />
            {frame.refusal && <Label at={geometry.callout(actor).refusal} text={frame.refusal} tone="refusal" glyph="✕" />}
          </>
        )}
        <InboxColumn frame={frame} />
      </g>
      {frame.token && (
        <Envelope
          key={`token-${key}`}
          className={`inbox-hero__token--moving${frame.removed ? ' inbox-hero__token--removed' : ''}`}
          style={style}
        />
      )}
    </g>
  );
}

function useMotion(): boolean {
  const [motion, setMotion] = useState(false);
  useEffect(() => {
    const query = window.matchMedia('(prefers-reduced-motion: reduce)');
    const update = () => setMotion(!query.matches);
    update();
    query.addEventListener('change', update);
    return () => query.removeEventListener('change', update);
  }, []);
  return motion;
}

function useFrame(running: boolean): number {
  const [index, setIndex] = useState(0);
  useEffect(() => {
    if (!running) return undefined;
    const timer = window.setInterval(() => setIndex((i) => (i + 1) % FRAMES.length), FRAME_MS);
    return () => window.clearInterval(timer);
  }, [running]);
  return index;
}

/** The wide layout, or the tall one where the figure is drawn narrower than TALL_BELOW_PX. */
function useLayout(figure: RefObject<HTMLElement | null>): LayoutName {
  const [name, setName] = useState<LayoutName>('wide');
  useEffect(() => {
    const node = figure.current;
    if (!node) return undefined;
    const update = () => setName(node.clientWidth > 0 && node.clientWidth < TALL_BELOW_PX ? 'tall' : 'wide');
    update();
    const observer = new ResizeObserver(update);
    observer.observe(node);
    return () => observer.disconnect();
  }, [figure]);
  return name;
}

function HostLabel({box}: {box: Box}) {
  return (
    <text x={box.x + 14} y={box.y - 10} className="inbox-hero__host-label">
      {LAYOUT.host}
    </text>
  );
}

function Topology({children}: {children?: ReactNode}) {
  const geometry = useGeometry();
  const {hosts, processes, agent} = geometry;
  const inbox = processes.inbox;
  const slot = geometry.slot;
  return (
    <>
      <Frame box={hosts.today} className="inbox-hero__host" label={<HostLabel box={hosts.today} />} />
      <g className="inbox-hero__planned" data-testid="hero-planned">
        <Link agent="planned" className="inbox-hero__link--planned" />
        <Frame box={hosts.planned} className="inbox-hero__host inbox-hero__host--planned" label={<HostLabel box={hosts.planned} />} />
        <Frame box={processes.planned} className="inbox-hero__process inbox-hero__process--planned" />
        <AgentGlyph at={agent.planned} className="inbox-hero__agent--planned" />
        <Label at={geometry.plannedLabel} text={LAYOUT.planned} tone="planned" />
      </g>
      <Link agent="sender" />
      <Link agent="recipient" />
      <Frame box={processes.sender} className="inbox-hero__process" />
      <Frame box={processes.recipient} className="inbox-hero__process" />
      <AgentGlyph at={agent.sender} />
      <AgentGlyph at={agent.recipient} />
      <Frame
        box={inbox}
        className="inbox-hero__process inbox-hero__process--inbox"
        label={
          <text x={geometry.inboxTitle.x} y={geometry.inboxTitle.y} textAnchor="middle" className="inbox-hero__inbox-label">
            {LAYOUT.service}
          </text>
        }
      />
      <g className="inbox-hero__tray" transform={`translate(${slot.x} ${slot.y + 4})`} aria-hidden="true">
        <path d="M -38 -8 L -26 -28 L 26 -28 L 38 -8 L 38 16 L -38 16 Z" />
        <path d="M -38 -8 L -15 -8 L -11 1 L 11 1 L 15 -8 L 38 -8" />
      </g>
      <g className="inbox-hero__legend" transform={`translate(${geometry.legend.x} ${geometry.legend.y})`}>
        <rect x={0} y={-14} width={26} height={18} rx={4} className="inbox-hero__legend-swatch" />
        <text x={36} y={0}>
          {LAYOUT.process}
        </text>
      </g>
      {children}
    </>
  );
}

const SUMMARY =
  `Agents, each in its own process, exchange messages through one ${String(data.inbox ?? 'inbox').toLowerCase()} service on one host. ` +
  `${data.planned_note ?? ''}`;

export default function InboxHero(): ReactNode {
  const figure = useRef<HTMLElement>(null);
  const layout = useLayout(figure);
  const geometry = LAYOUTS[layout];
  const reduced = !useMotion();
  const motion = !reduced && FRAMES.length > 0;
  const index = useFrame(motion);
  const frame = FRAMES[index % Math.max(FRAMES.length, 1)];
  return (
    <figure ref={figure} className="inbox-hero" data-testid="inbox-hero" data-mode={motion ? 'animated' : 'still'} data-layout={layout}>
      <GeometryContext.Provider value={geometry}>
        <svg viewBox={`0 0 ${geometry.view.w} ${geometry.view.h}`} role="img" aria-labelledby="inbox-hero-title" className="inbox-hero__svg">
          <title id="inbox-hero-title">{SUMMARY}</title>
          <Topology>{motion && frame ? <Moving frame={frame} previous={FRAMES[index - 1]} /> : <Still />}</Topology>
        </svg>
      </GeometryContext.Provider>
    </figure>
  );
}

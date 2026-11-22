import type {ReactNode} from 'react';
import animation from '@site/src/generated/animation.json';

/** Every graphic is 360 units wide and writes text at 16 units: at least 13 px where it is drawn 300 px wide. */
const W = 360;
const FONT = 16;
const MONO = FONT * 0.62;

function Figure({label, height, children}: {label: string; height: number; children: ReactNode}) {
  return (
    <svg className="inbox-pitch__svg" viewBox={`0 0 ${W} ${height}`} role="img" aria-label={label}>
      {children}
    </svg>
  );
}

function Pill({x, y, text, tone = 'plain', width}: {x: number; y: number; text: string; tone?: string; width?: number}) {
  const w = width ?? Math.round(text.length * MONO + 28);
  return (
    <g className={`inbox-pitch__pill inbox-pitch__pill--${tone}`} transform={`translate(${x - w / 2} ${y})`}>
      <rect x={0} y={-15} width={w} height={30} rx={15} />
      <text x={w / 2} y={5.5} textAnchor="middle">
        {text}
      </text>
    </g>
  );
}

function Arrow({x1, y1, x2, y2}: {x1: number; y1: number; x2: number; y2: number}) {
  const angle = Math.atan2(y2 - y1, x2 - x1);
  const head = 8;
  const a = `${x2 - head * Math.cos(angle - 0.45)},${y2 - head * Math.sin(angle - 0.45)}`;
  const b = `${x2 - head * Math.cos(angle + 0.45)},${y2 - head * Math.sin(angle + 0.45)}`;
  return (
    <g className="inbox-pitch__arrow">
      <line x1={x1} y1={y1} x2={x2} y2={y2} />
      <polyline points={`${a} ${x2},${y2} ${b}`} />
    </g>
  );
}

/** The command that leads into a state, read from the frames of the generated data. */
function commandInto(state: string): string {
  return animation.frames.find((frame) => frame.state === state && frame.actor !== 'service')?.command ?? '';
}

/** The lifecycle of a message as the specification declares it: the first path, the command into each state, then the terminal states. */
export function LifecycleGraphic(): ReactNode {
  const {path, terminal} = animation.lifecycle;
  const x = W / 2;
  const top = 58;
  const step = 72;
  const rows = path.map((state, i) => ({state, y: top + i * step, command: commandInto(state)}));
  const last = rows[rows.length - 1];
  const endY = last.y + 92;
  const caption = endY + 52;
  return (
    <Figure
      height={caption + 8}
      label={`A message is ${path.join(', then ')}, and ends ${terminal.join(' or ')}; each step is a command of the specification.`}>
      {rows.map((row, i) => (
        <g key={row.state}>
          <Arrow x1={x} y1={i === 0 ? 4 : rows[i - 1].y + 16} x2={x} y2={row.y - 17} />
          <text className="inbox-pitch__command" x={x + 12} y={row.y - step / 2 + 6}>
            {row.command}
          </text>
          <Pill x={x} y={row.y} text={row.state} tone="state" />
        </g>
      ))}
      {terminal.map((state, i) => {
        const side = i === 0 ? -1 : 1;
        const to = x + side * 92;
        return (
          <g key={state}>
            <Arrow x1={x + side * 14} y1={last.y + 16} x2={to - side * 10} y2={endY - 17} />
            <text className="inbox-pitch__command" x={to + side * 14} y={last.y + 44} textAnchor={side < 0 ? 'end' : 'start'}>
              {commandInto(state)}
            </text>
            <Pill x={to} y={endY} text={state} tone="end" />
          </g>
        );
      })}
      <text className="inbox-pitch__caption" x={x} y={caption} textAnchor="middle">
        {`${animation.counts.commands} commands · ${animation.counts.views} views · ${animation.counts.errors} refusals`}
      </text>
    </Figure>
  );
}

function Client({x, y}: {x: number; y: number}) {
  return (
    <g className="inbox-pitch__agent" transform={`translate(${x} ${y})`}>
      <rect className="inbox-pitch__process" x={-34} y={-30} width={68} height={60} rx={10} />
      <circle r={18} />
      <rect className="inbox-pitch__face" x={-9} y={-6} width={18} height={12} rx={3} />
    </g>
  );
}

/** Clients talk to one service; the backend sits behind one contract. */
export function ShapeGraphic(): ReactNode {
  const clients = [60, 180, 300];
  return (
    <Figure height={330} label="Every client talks to the one service; the backend sits behind one storage contract and its test kit.">
      {clients.map((x) => (
        <g key={x}>
          <Client x={x} y={34} />
          <Arrow x1={x} y1={66} x2={180 + (x - 180) * 0.45} y2={110} />
        </g>
      ))}
      <g className="inbox-pitch__service">
        <rect x={30} y={114} width={300} height={62} rx={12} />
        <text className="inbox-pitch__box-title" x={180} y={140} textAnchor="middle">
          inbox serve
        </text>
        <text className="inbox-pitch__box-sub" x={180} y={164} textAnchor="middle">
          {`${animation.counts.commands} commands · ${animation.counts.views} views`}
        </text>
      </g>
      <Arrow x1={180} y1={178} x2={180} y2={210} />
      <Pill x={180} y={228} text="storage contract + test kit" tone="contract" />
      <line className="inbox-pitch__rule" x1={180} y1={244} x2={180} y2={268} />
      <Pill x={180} y={286} text="in-memory backend" />
      <text className="inbox-pitch__caption" x={180} y={326} textAnchor="middle">
        no client talks to a backend
      </text>
    </Figure>
  );
}

/** The conformance suite against the core and over HTTP, and the open findings. */
export function SuiteGraphic({total, core, http, findings}: {total: number; core: number; http: number; findings: number}): ReactNode {
  const bar = (y: number, passed: number, label: string, value: string) => (
    <g className="inbox-pitch__bar" transform={`translate(0 ${y})`}>
      <text className="inbox-pitch__bar-label" x={0} y={0}>
        {label}
      </text>
      <text className="inbox-pitch__bar-value" x={W} y={0} textAnchor="end">
        {value}
      </text>
      <rect className="inbox-pitch__bar-track" x={1} y={12} width={W - 2} height={22} rx={6} />
      <rect className="inbox-pitch__bar-fill" x={1} y={12} width={Math.max(((W - 2) * passed) / total, 12)} height={22} rx={6} />
    </g>
  );
  return (
    <Figure
      height={262}
      label={`${core} of ${total} scenarios pass against the core; over HTTP ${http} of ${total} can be witnessed, and pass; ${findings} security findings are open.`}>
      {bar(24, core, 'against the core', `${core} of ${total}`)}
      {bar(104, http, 'over HTTP', `${http} witnessed`)}
      <g transform="translate(0 184)">
        <text className="inbox-pitch__bar-label" x={0} y={0}>
          security findings
        </text>
        {Array.from({length: findings}, (_, i) => (
          <g key={i} className="inbox-pitch__finding" transform={`translate(${18 + i * 44} 40)`}>
            <circle r={16} />
            <text y={5.5} textAnchor="middle">
              {i + 1}
            </text>
          </g>
        ))}
        <text className="inbox-pitch__bar-value" x={W} y={46} textAnchor="end">
          {`${findings} open`}
        </text>
      </g>
    </Figure>
  );
}

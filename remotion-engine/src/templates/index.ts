export { AnimatedText } from './AnimatedText';
export type { AnimatedTextProps } from './AnimatedText';
export { CallToAction } from './CallToAction';
export type { CallToActionProps } from './CallToAction';
export { Comparison } from './Comparison';
export type { ComparisonProps } from './Comparison';
export { Counter } from './Counter';
export type { CounterProps } from './Counter';
export { DataChart } from './DataChart';
export type { DataChartProps } from './DataChart';
export { LogoReveal } from './LogoReveal';
export type { LogoRevealProps } from './LogoReveal';
export { LowerThird } from './LowerThird';
export type { LowerThirdProps } from './LowerThird';
export { ProgressBar } from './ProgressBar';
export type { ProgressBarProps } from './ProgressBar';
export { ScreenFrame } from './ScreenFrame';
export type { ScreenFrameProps } from './ScreenFrame';
export { SocialProof } from './SocialProof';
export type { SocialProofProps } from './SocialProof';
export { ZoomPan } from './ZoomPan';
export type { ZoomPanProps } from './ZoomPan';

export const MOTION_TEMPLATES = {
    'animated-text': {
        name: 'Animated Text',
        description: 'Text with various animation styles',
        component: 'AnimatedText',
        category: 'text',
        defaultProps: { text: 'Your Text Here', style: 'typewriter', color: '#ffffff', fontSize: 64 },
        styles: ['typewriter', 'bounce', 'fade-up', 'word-by-word', 'glitch'],
    },
    'lower-third': {
        name: 'Lower Third',
        description: 'Name/title bar overlay',
        component: 'LowerThird',
        category: 'overlay',
        defaultProps: { name: 'John Doe', title: 'CEO', style: 'modern', primaryColor: '#f97316' },
        styles: ['modern', 'minimal', 'gradient', 'glassmorphism'],
    },
    'call-to-action': {
        name: 'Call to Action',
        description: 'Animated CTA button',
        component: 'CallToAction',
        category: 'overlay',
        defaultProps: { type: 'subscribe', style: 'pill', primaryColor: '#ef4444' },
        styles: ['pill', 'box', 'floating', 'pulse'],
    },
    'counter': {
        name: 'Counter',
        description: 'Animated number counter',
        component: 'Counter',
        category: 'data',
        defaultProps: { from: 0, to: 1000, suffix: '+', label: 'Users', color: '#f97316' },
    },
    'logo-reveal': {
        name: 'Logo Reveal',
        description: 'Logo with reveal animation',
        component: 'LogoReveal',
        category: 'branding',
        defaultProps: { logoUrl: '', style: 'fade-scale', backgroundColor: '#0a0a0a' },
        styles: ['fade-scale', 'glitch', 'particles', 'morph'],
    },
    'screen-frame': {
        name: 'Screen Frame',
        description: 'Device frame for screenshots',
        component: 'ScreenFrame',
        category: 'media',
        defaultProps: { device: 'macbook', screenshotUrl: '' },
        styles: ['macbook', 'iphone', 'browser', 'minimal'],
    },
    'social-proof': {
        name: 'Social Proof',
        description: 'Testimonial or review card',
        component: 'SocialProof',
        category: 'text',
        defaultProps: { quote: 'Amazing product!', author: 'Jane D.', rating: 5, style: 'card' },
        styles: ['card', 'minimal', 'tweet', 'review'],
    },
    'progress-bar': {
        name: 'Progress Bar',
        description: 'Animated progress indicator',
        component: 'ProgressBar',
        category: 'data',
        defaultProps: { value: 75, maxValue: 100, label: 'Progress', color: '#22c55e' },
    },
    'comparison': {
        name: 'Comparison',
        description: 'Before/after comparison',
        component: 'Comparison',
        category: 'media',
        defaultProps: { type: 'slider', beforeLabel: 'Before', afterLabel: 'After' },
        styles: ['slider', 'side-by-side', 'flip', 'fade'],
    },
    'zoom-pan': {
        name: 'Zoom & Pan',
        description: 'Ken Burns zoom and pan effect',
        component: 'ZoomPan',
        category: 'media',
        defaultProps: { imageUrl: '', style: 'zoom-in', duration: 90 },
        styles: ['zoom-in', 'zoom-out', 'pan-left', 'pan-right', 'ken-burns'],
    },
    'data-chart': {
        name: 'Data Chart',
        description: 'Animated chart visualization',
        component: 'DataChart',
        category: 'data',
        defaultProps: { type: 'bar', data: [{ label: 'A', value: 80 }, { label: 'B', value: 60 }] },
        styles: ['bar', 'pie', 'progress', 'line'],
    },
} as const;

export type TemplateId = keyof typeof MOTION_TEMPLATES;

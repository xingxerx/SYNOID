import React from 'react';
import { Composition } from 'remotion';
import { DynamicAnimation } from './DynamicAnimation';

// Props passed from the CLI via --props
export interface DynamicAnimationProps {
    scenes: Scene[];
    title?: string;
    backgroundColor?: string;
    totalDuration?: number;
}

export interface ShapeConfig {
    type: 'circle' | 'rect' | 'triangle' | 'star' | 'polygon' | 'ellipse';
    fill?: string;
    stroke?: string;
    strokeWidth?: number;
    x?: number;
    y?: number;
    scale?: number;
    rotation?: number;
    delay?: number;
    radius?: number;
    width?: number;
    height?: number;
    cornerRadius?: number;
    length?: number;
    direction?: 'up' | 'down' | 'left' | 'right';
    points?: number;
    innerRadius?: number;
    outerRadius?: number;
    rx?: number;
    ry?: number;
    animation?: 'none' | 'pop' | 'spin' | 'bounce' | 'float' | 'pulse' | 'draw';
}

export interface EmojiConfig {
    emoji: string;
    x?: number;
    y?: number;
    scale?: number;
    delay?: number;
    animation?: 'none' | 'pop' | 'bounce' | 'float' | 'pulse' | 'spin' | 'shake' | 'wave';
}

export interface GifConfig {
    src: string;
    x?: number;
    y?: number;
    width?: number;
    height?: number;
    scale?: number;
    delay?: number;
    loop?: boolean;
    fit?: 'fill' | 'contain' | 'cover';
    playbackRate?: number;
    animation?: 'none' | 'pop' | 'bounce' | 'float' | 'pulse' | 'spin' | 'shake';
}

export interface LottieConfig {
    src: string;
    x?: number;
    y?: number;
    width?: number;
    height?: number;
    scale?: number;
    delay?: number;
    loop?: boolean;
    playbackRate?: number;
    direction?: 'forward' | 'backward';
}

export interface Scene {
    id: string;
    type: 'title' | 'steps' | 'features' | 'stats' | 'text' | 'transition' | 'media' | 'chart' | 'countdown' | 'comparison' | 'shapes' | 'emoji' | 'gif' | 'lottie';
    duration: number;
    content: SceneContent;
    transition?: {
        type: 'none' | 'fade' | 'swipe-left' | 'swipe-right' | 'swipe-up' | 'swipe-down' | 'zoom-in' | 'zoom-out' | 'wipe-left' | 'wipe-right' | 'blur' | 'flip';
        duration?: number;
    };
}

export interface SceneContent {
    title?: string;
    subtitle?: string;
    items?: Array<{
        icon?: string;
        label: string;
        description?: string;
        value?: number;
        color?: string;
    }>;
    stats?: Array<{
        value: string;
        label: string;
        numericValue?: number;
        prefix?: string;
        suffix?: string;
    }>;
    color?: string;
    backgroundColor?: string;
    shapes?: ShapeConfig[];
    shapesLayout?: 'scattered' | 'grid' | 'circle' | 'custom';
    emojis?: EmojiConfig[];
    emojiLayout?: 'scattered' | 'grid' | 'circle' | 'row' | 'custom';
    gifs?: GifConfig[];
    gifLayout?: 'scattered' | 'grid' | 'circle' | 'row' | 'fullscreen' | 'custom';
    gifBackground?: string;
    lotties?: LottieConfig[];
    lottieLayout?: 'scattered' | 'grid' | 'circle' | 'row' | 'fullscreen' | 'custom';
    lottieBackground?: string;
    camera?: {
        type: 'zoom-in' | 'zoom-out' | 'pan-left' | 'pan-right' | 'pan-up' | 'pan-down' | 'ken-burns' | 'shake';
        intensity?: number;
    };
    mediaAssetId?: string;
    mediaPath?: string;
    mediaType?: 'image' | 'video';
    mediaStyle?: 'fullscreen' | 'framed' | 'pip' | 'background' | 'split-left' | 'split-right' | 'circle' | 'phone-frame';
    videoStartFrom?: number;
    videoEndAt?: number;
    videoVolume?: number;
    videoPlaybackRate?: number;
    videoLoop?: boolean;
    videoMuted?: boolean;
    mediaAnimation?: {
        type: 'none' | 'ken-burns' | 'zoom-in' | 'zoom-out' | 'pan-left' | 'pan-right' | 'pan-up' | 'pan-down' | 'rotate' | 'parallax';
        intensity?: number;
    };
    overlayText?: string;
    overlayPosition?: 'top' | 'center' | 'bottom';
    overlayStyle?: 'minimal' | 'bold' | 'gradient-bar';
    chartType?: 'bar' | 'progress' | 'pie' | 'line';
    chartData?: Array<{ label: string; value: number; color?: string }>;
    maxValue?: number;
    countFrom?: number;
    countTo?: number;
    beforeLabel?: string;
    afterLabel?: string;
    beforeValue?: string;
    afterValue?: string;
    beforeMedia?: string;
    afterMedia?: string;
}

// Calculate duration from scenes
const calculateDuration = (props: DynamicAnimationProps): number => {
    if (props.totalDuration) {
        return props.totalDuration;
    }
    if (props.scenes && props.scenes.length > 0) {
        return props.scenes.reduce((sum, scene) => sum + (scene.duration || 60), 0);
    }
    return 300; // Default fallback
};

export const RemotionRoot: React.FC = () => {
    return (
        <>
            <Composition
                id="DynamicAnimation"
                component={DynamicAnimation}
                durationInFrames={300}
                fps={30}
                width={1920}
                height={1080}
                defaultProps={{
                    scenes: [],
                    backgroundColor: '#0a0a0a',
                }}
                calculateMetadata={({ props }) => {
                    return {
                        durationInFrames: calculateDuration(props),
                    };
                }}
            />
        </>
    );
};

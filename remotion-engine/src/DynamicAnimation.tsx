import React from 'react';
import { AbsoluteFill, Sequence } from 'remotion';
import { SceneRenderer } from './scenes';
import type { DynamicAnimationProps } from './Root';

/**
 * Main DynamicAnimation component.
 * Renders a sequence of scenes, each with its own type, duration,
 * transitions, and camera effects.
 *
 * This is the composition registered in Root.tsx and rendered by the CLI.
 */
export const DynamicAnimation: React.FC<DynamicAnimationProps> = ({
    scenes,
    backgroundColor = '#0a0a0a',
}) => {
    let frameOffset = 0;

    return (
        <AbsoluteFill style={{ backgroundColor }}>
            {scenes.map((scene, index) => {
                const from = frameOffset;
                frameOffset += scene.duration;

                return (
                    <Sequence
                        key={scene.id || `scene-${index}`}
                        from={from}
                        durationInFrames={scene.duration}
                    >
                        <SceneRenderer scene={scene} />
                    </Sequence>
                );
            })}
        </AbsoluteFill>
    );
};

export default DynamicAnimation;

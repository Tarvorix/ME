/**
 * Sound manager using Howler.js for game audio.
 * Provides spatial audio (volume by distance), event-driven sound effects,
 * background music, and volume controls.
 *
 * iOS requires user gesture to unlock AudioContext — handled via
 * unlock-on-first-interaction pattern.
 */

import { Howl, Howler } from 'howler';

/** Spatial audio configuration */
const SPATIAL_MAX_DISTANCE = 40.0;
const SPATIAL_ROLLOFF = 1.0;

/** Default volume levels */
const DEFAULT_MASTER_VOLUME = 0.8;
const DEFAULT_SFX_VOLUME = 0.7;
const DEFAULT_MUSIC_VOLUME = 0.4;

/** Sound effect categories */
export type SfxType =
    | 'shot'
    | 'death'
    | 'spawn'
    | 'captureComplete'
    | 'click'
    | 'select'
    | 'moveOrder'
    | 'productionComplete'
    | 'battleEnd';

/**
 * Computes volume based on distance from the listener (camera center).
 * Returns a value in [0, 1] using linear falloff.
 */
export function spatialVolume(
    soundX: number,
    soundY: number,
    listenerX: number,
    listenerY: number,
    maxDistance: number = SPATIAL_MAX_DISTANCE,
): number {
    const dx = soundX - listenerX;
    const dy = soundY - listenerY;
    const dist = Math.sqrt(dx * dx + dy * dy);

    if (dist >= maxDistance) return 0.0;
    if (dist <= 0) return 1.0;

    return Math.max(0, 1.0 - (dist / maxDistance) * SPATIAL_ROLLOFF);
}

/**
 * Pan value based on horizontal offset from listener center.
 * Returns [-1, 1] where -1 is full left and 1 is full right.
 */
export function spatialPan(
    soundX: number,
    listenerX: number,
    maxDistance: number = SPATIAL_MAX_DISTANCE,
): number {
    const offset = soundX - listenerX;
    return Math.max(-1, Math.min(1, offset / maxDistance));
}

/**
 * Sound manager singleton.
 * Lazily initializes Howler sounds and manages playback.
 */
export class SoundManager {
    private initialized = false;
    private unlocked = false;

    private masterVolume = DEFAULT_MASTER_VOLUME;
    private sfxVolume = DEFAULT_SFX_VOLUME;
    private musicVolume = DEFAULT_MUSIC_VOLUME;

    private listenerX = 32.0;
    private listenerY = 32.0;

    private sounds: Map<string, Howl> = new Map();
    private musicTrack: Howl | null = null;
    private musicId: number | null = null;

    constructor() {
        // Bind unlock handler
        this.tryUnlock = this.tryUnlock.bind(this);
    }

    /**
     * Initialize the sound manager. Call this once after page load.
     * Sets up the iOS audio unlock handler.
     */
    init(): void {
        if (this.initialized) return;
        this.initialized = true;

        // iOS/Safari audio unlock on first user gesture
        const unlockEvents = ['touchstart', 'touchend', 'mousedown', 'keydown'];
        for (const event of unlockEvents) {
            document.addEventListener(event, this.tryUnlock, { once: false, passive: true });
        }
    }

    /**
     * Attempt to unlock audio context on first user interaction.
     */
    private tryUnlock(): void {
        if (this.unlocked) return;
        this.unlocked = true;

        // Remove all unlock listeners
        const unlockEvents = ['touchstart', 'touchend', 'mousedown', 'keydown'];
        for (const event of unlockEvents) {
            document.removeEventListener(event, this.tryUnlock);
        }

        // Try to resume AudioContext if Howler has one suspended
        try {
            if (Howler.ctx && Howler.ctx.state === 'suspended') {
                Howler.ctx.resume();
            }
        } catch {
            // Ignore errors
        }
    }

    /**
     * Update the listener position (typically the camera center in tile coords).
     */
    setListenerPosition(x: number, y: number): void {
        this.listenerX = x;
        this.listenerY = y;
    }

    /**
     * Set master volume (0-1). Affects both SFX and music.
     */
    setMasterVolume(vol: number): void {
        this.masterVolume = Math.max(0, Math.min(1, vol));
    }

    /**
     * Set SFX volume (0-1).
     */
    setSfxVolume(vol: number): void {
        this.sfxVolume = Math.max(0, Math.min(1, vol));
    }

    /**
     * Set music volume (0-1).
     */
    setMusicVolume(vol: number): void {
        this.musicVolume = Math.max(0, Math.min(1, vol));
        if (this.musicTrack && this.musicId !== null) {
            this.musicTrack.volume(this.musicVolume * this.masterVolume, this.musicId);
        }
    }

    /**
     * Get a Howl instance for the given sound, creating it if needed.
     */
    private getSound(name: string, src: string): Howl {
        let sound = this.sounds.get(name);
        if (!sound) {
            sound = new Howl({
                src: [src],
                preload: true,
                volume: 1.0,
            });
            this.sounds.set(name, sound);
        }
        return sound;
    }

    /**
     * Play a sound effect with spatial audio based on tile position.
     */
    playSfxAt(type: SfxType, tileX: number, tileY: number): void {
        const vol = spatialVolume(tileX, tileY, this.listenerX, this.listenerY);
        if (vol <= 0.01) return; // Too far away to hear

        const effectiveVol = vol * this.sfxVolume * this.masterVolume;
        const pan = spatialPan(tileX, this.listenerX);

        const src = this.sfxSourcePath(type);
        const sound = this.getSound(type, src);

        const id = sound.play();
        sound.volume(effectiveVol, id);
        sound.stereo(pan, id);
    }

    /**
     * Play a non-spatial UI sound effect (click, select, etc.).
     */
    playSfx(type: SfxType): void {
        const effectiveVol = this.sfxVolume * this.masterVolume;
        if (effectiveVol <= 0.01) return;

        const src = this.sfxSourcePath(type);
        const sound = this.getSound(type, src);

        const id = sound.play();
        sound.volume(effectiveVol, id);
    }

    /**
     * Event-driven sound triggers — called from game renderer on events.
     */
    playShot(x: number, y: number): void {
        this.playSfxAt('shot', x, y);
    }

    playDeath(x: number, y: number): void {
        this.playSfxAt('death', x, y);
    }

    playSpawn(x: number, y: number): void {
        this.playSfxAt('spawn', x, y);
    }

    playCaptureComplete(x: number, y: number): void {
        this.playSfxAt('captureComplete', x, y);
    }

    playProductionComplete(): void {
        this.playSfx('productionComplete');
    }

    playBattleEnd(): void {
        this.playSfx('battleEnd');
    }

    playClick(): void {
        this.playSfx('click');
    }

    playSelect(): void {
        this.playSfx('select');
    }

    playMoveOrder(): void {
        this.playSfx('moveOrder');
    }

    /**
     * Start playing background music. Loops indefinitely.
     */
    playMusic(trackSrc: string): void {
        // Stop existing music
        this.stopMusic();

        this.musicTrack = new Howl({
            src: [trackSrc],
            loop: true,
            volume: this.musicVolume * this.masterVolume,
        });

        this.musicId = this.musicTrack.play();
    }

    /**
     * Stop the currently playing music track.
     */
    stopMusic(): void {
        if (this.musicTrack) {
            this.musicTrack.stop();
            this.musicTrack.unload();
            this.musicTrack = null;
            this.musicId = null;
        }
    }

    /**
     * Pause the currently playing music.
     */
    pauseMusic(): void {
        if (this.musicTrack && this.musicId !== null) {
            this.musicTrack.pause(this.musicId);
        }
    }

    /**
     * Resume paused music.
     */
    resumeMusic(): void {
        if (this.musicTrack && this.musicId !== null) {
            this.musicTrack.play(this.musicId);
        }
    }

    /**
     * Get the source path for a sound effect type.
     * Audio files should be placed in /assets/audio/ or /public/audio/.
     */
    private sfxSourcePath(type: SfxType): string {
        const map: Record<SfxType, string> = {
            shot: '/assets/audio/shot.mp3',
            death: '/assets/audio/death.mp3',
            spawn: '/assets/audio/spawn.mp3',
            captureComplete: '/assets/audio/capture_complete.mp3',
            click: '/assets/audio/click.mp3',
            select: '/assets/audio/select.mp3',
            moveOrder: '/assets/audio/move_order.mp3',
            productionComplete: '/assets/audio/production_complete.mp3',
            battleEnd: '/assets/audio/battle_end.mp3',
        };
        return map[type];
    }

    /**
     * Clean up all sounds and release resources.
     */
    destroy(): void {
        this.stopMusic();
        for (const sound of this.sounds.values()) {
            sound.unload();
        }
        this.sounds.clear();
    }
}


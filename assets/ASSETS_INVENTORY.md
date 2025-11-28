# Assets Inventory

Complete listing of all game assets and their purposes.

---

## 🎵 Audio Assets

### Music (`audio/music/`)
- **`menu.mp3`** - Main menu background music (loop)

### Sound Effects (`audio/sfx/`)
- **`button_tap.wav`** - UI button clicks and interactions
- **`tile_swap.wav`** - Sound when swapping tiles
- **`match_3.mp3/.wav`** - 3-tile match sound
- **`match_4.wav`** - 4-tile match sound
- **`match_5.wav`** - 5+ tile match sound
- **`combo_1.wav`** - Combo level 1 sound
- **`combo_2.wav`** - Combo level 2 sound
- **`combo_3.wav`** - Combo level 3+ sound
- **`powerup_activate.wav`** - Power-up activation sound
- **`blocker_hit.wav`** - Sound when blocker takes damage
- **`blocker_break.wav`** - Sound when blocker is destroyed
- **`level_complete.wav`** - Level completion celebration
- **`level_fail.wav`** - Level failure sound
- **`matchmaking.wav`** - Matchmaking found notification
- **`countdown.wav`** - Match countdown timer beep

---

## 🖼️ Image Assets

### Backgrounds (`images/backgrounds/`)
- **`menu-bg.jpg`** - Main menu background
- **`game-bg.jpg`** - In-game background
- **`victory-bg.jpeg`** - Victory screen background
- **`defeat-bg.jpg`** - Defeat/game over screen background

### Boosters (`images/boosters/`)
Construction-themed power-up items:
- **`jackhammer.png`** - Jackhammer booster
- **`drill.png`** - Drill booster
- **`mixer.png`** - Cement mixer booster
- **`bulldozer.png`** - Bulldozer booster
- **`wrecking_ball.png`** - Wrecking ball booster
- **`shield_crane.png`** - Shield crane booster
- **`barrel.png`** - Barrel booster
- **`breakerjack.png`** - Breaker jack booster

### Tiles (`images/tiles/`)
Game board pieces:
- **`color-0.png`** - Colored tile type 1
- **`color-1.png`** - Colored tile type 2
- **`color-2.png`** - Colored tile type 3
- **`color-3.png`** - Colored tile type 4
- **`color-4.png`** - Colored tile type 5
- **`color-5.png`** - Colored tile type 6
- **`color-empty.png`** - Empty tile slot
- **`blocker-1hp.png`** - Single health blocker tile
- **`blocker-2hp.png`** - Double health blocker tile

---

## ⚡ Power-ups (`powerups/`)
In-game power-up items:
- **`jackhammer.png`** - Jackhammer power-up
- **`drill.png`** - Drill power-up
- **`mini-drill.png`** - Mini drill power-up
- **`mixer.png`** - Mixer power-up
- **`bulldozer.png`** - Bulldozer power-up
- **`wrecking_ball.png`** - Wrecking ball power-up
- **`shield_crane.png`** - Shield crane power-up
- **`barrel.png`** - Barrel power-up
- **`breakerjack.png`** - Breaker jack power-up
- **`micro-refill.png`** - Micro refill power-up
- **`soft-shuffle.png`** - Soft shuffle power-up
- **`tile_pulse.png`** - Tile pulse effect power-up

---

## 🎨 UI Assets (`ui/`)

### Icons (`ui/icons/`)
Currently available:
- **`play_outline.svg`** - Play button
- **`close_outline.svg`** - Close/dismiss button
- **`settings_outline.svg`** - Settings icon
- **`star_filled.svg`** - Star rating/favorites
- **`coin_filled.svg`** - Standard currency (coins)
- **`gem_filled.svg`** - Premium currency (gems)
- **`lightning_filled.svg`** - Energy/power indicator

### Planned Icons (from `ui_assets.yaml`)

#### Outline Style Icons (24x24dp)
- `sound_outline.svg` - Sound effects toggle
- `music_outline.svg` - Background music toggle
- `vibration_outline.svg` - Haptic feedback toggle
- `shop_outline.svg` - In-game shop
- `map_outline.svg` - World map / city view
- `home_outline.svg` - Home / main menu
- `pause_outline.svg` - Pause button
- `back_outline.svg` - Back navigation
- `info_outline.svg` - Information / help
- `leaderboard_outline.svg` - Leaderboard / rankings
- `trophy_outline.svg` - Achievements
- `gift_outline.svg` - Rewards / gifts
- `calendar_outline.svg` - Daily rewards / events

#### Filled Style Icons (24x24dp)
- `crown_filled.svg` - Premium / VIP status
- `trophy_filled.svg` - Victory / achievement
- `heart_filled.svg` - Lives / health
- `key_filled.svg` - Unlock / access
- `chest_filled.svg` - Treasure / loot box
- `check_filled.svg` - Success / complete
- `cross_filled.svg` - Error / cancel
- `shield_filled.svg` - Protection / defense
- `hammer_filled.svg` - Power-up / tool
- `bomb_filled.svg` - Explosive power-up

### Planned Backgrounds
- `gradient_sky.png` - Blue sky gradient (1125x2436 @1x)
- `soft_clouds.png` - Cloud texture overlay (transparent PNG)
- `card_paper.9.png` - 9-patch card background texture
- `ui_shadow.png` - Soft drop shadow for elevated elements
- `sparkle_particle.png` - Sparkle effect particle
- `glow_radial.png` - Radial glow effect

### Planned Button Assets
- `button_gloss_overlay.svg` - Glossy specular highlight overlay
- `button_inner_shadow.svg` - Subtle inner shadow for depth

### Planned UI Decorations
- `corner_ornament_tl.svg` - Top-left corner decoration
- `corner_ornament_tr.svg` - Top-right corner decoration
- `divider_ornate.svg` - Decorative divider line
- `ribbon_premium.svg` - Premium ribbon badge

---

## 🎮 Game Theme

This is a **construction/building-themed match-3 puzzle game** featuring:
- Match-3 tile gameplay with 6 color types
- Construction-themed power-ups and boosters
- Blocker obstacles with health points
- Combo system (3 levels)
- Dual currency system (coins and gems)
- Energy/lives system
- Multiplayer matchmaking support
- Level progression with victory/defeat screens

---

## 📐 Design Specifications

### Icon Specifications
- **Size**: 24x24dp (72x72px @3x)
- **Stroke Width**: 2px
- **Style**: Rounded, friendly, simple
- **Colors**: Single color (programmatically tinted)

### Background Sizes
- **1x**: 1125x2436px (iPhone X/11 Pro)
- **2x**: 2250x4872px
- **3x**: 3375x7308px

### Color Palette
- **Gold**: `#FFD700`
- **Blue**: `#2196F3`
- **Green**: `#32CD32`
- **Red**: `#FF3D3D`
- **Purple**: `#BA55D3`

### Corner Radii
- **xs**: 8dp
- **sm**: 12dp
- **md**: 16dp
- **lg**: 20dp
- **xl**: 28dp
- **2xl**: 36dp

### Shadows
- **Card**: `0 2dp 6dp rgba(0,0,0,0.08)`
- **Button**: `0 4dp 8dp rgba(0,0,0,0.15)`
- **Chip**: `0 2dp 4dp rgba(0,0,0,0.06)`

---

*Last Updated: November 28, 2025*

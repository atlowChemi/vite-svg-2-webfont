<script lang="ts">
    import { iconBaseSelector, iconClassPrefix, type IconName } from 'virtual:typed-icons';

    let { name }: { name: IconName } = $props();
</script>

<i class="{iconBaseSelector.slice(1)} {iconClassPrefix}{name}" aria-hidden="true"></i>

<script setup lang="ts">
import { iconBaseSelector, iconClassPrefix, type IconName } from './icons';

defineProps<{ name: IconName }>();
</script>

<template>
    <i :class="[iconBaseSelector.slice(1), `${iconClassPrefix}${name}`]" aria-hidden="true" />
</template>

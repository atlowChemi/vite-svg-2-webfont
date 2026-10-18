import { Component, computed, input } from '@angular/core';
import { iconBaseSelector, iconClassPrefix, type IconName } from './icons';

@Component({
    selector: 'app-icon',
    template: '<i [class]="classes()" aria-hidden="true"></i>',
})
export class IconComponent {
    readonly name = input.required<IconName>();
    protected readonly classes = computed(() => `${iconBaseSelector.slice(1)} ${iconClassPrefix}${this.name()}`);
}

// ECharts 按需导入配置
import { use } from "echarts/core";
import { CanvasRenderer } from "echarts/renderers";
import { LineChart } from "echarts/charts";
import {
    GridComponent,
    TooltipComponent,
    DataZoomComponent,
    MarkLineComponent,
    LegendComponent,
} from "echarts/components";

// 注册 ECharts 组件
use([
    CanvasRenderer,
    LineChart,
    GridComponent,
    TooltipComponent,
    DataZoomComponent,
    MarkLineComponent,
    LegendComponent,
]);

// 导出配置好的 VChart 组件
export { default as VChart } from "vue-echarts";

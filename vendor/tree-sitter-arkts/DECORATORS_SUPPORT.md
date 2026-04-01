# ArkTS 装饰器完整支持列表

tree-sitter-arkts 解析器现已完整支持 HarmonyOS ArkTS 的所有官方装饰器。

## 📋 支持的装饰器类型

### 🔷 基础装饰器
| 装饰器 | 说明 | 适用范围 |
|--------|------|----------|
| `@Entry` | 标记页面入口组件 | 组件 |
| `@Component` | 声明自定义组件（V1） | 组件 |
| `@ComponentV2` | 声明自定义组件（V2新架构） | 组件 |

### 🔷 状态管理 V1 装饰器
| 装饰器 | 说明 | 适用范围 |
|--------|------|----------|
| `@State` | 组件内部状态，双向绑定 | 属性 |
| `@Prop` | 父子单向同步（父→子） | 属性 |
| `@Link` | 父子双向同步 | 属性 |
| `@Provide` | 与后代组件双向同步（提供方） | 属性 |
| `@Consume` | 与后代组件双向同步（消费方） | 属性 |
| `@ObjectLink` | 嵌套对象双向同步 | 属性 |
| `@Observed` | 类对象观测装饰器 | 类 |
| `@Watch` | 状态变化监听回调 | 属性 |
| `@StorageLink` | AppStorage 双向同步 | 属性 |
| `@StorageProp` | AppStorage 单向同步 | 属性 |
| `@LocalStorageLink` | LocalStorage 双向同步 | 属性 |
| `@LocalStorageProp` | LocalStorage 单向同步 | 属性 |
| `@Track` | 精细化属性观测 | 类属性 |

### 🔷 状态管理 V2 装饰器（新架构）
| 装饰器 | 说明 | 适用范围 |
|--------|------|----------|
| `@Local` | 组件内部状态（V2） | 属性 |
| `@Param` | 组件外部输入（V2） | 属性 |
| `@Once` | 初始化同步一次 | 属性 |
| `@Event` | 规范组件输出事件 | 属性 |
| `@Provider` | 跨组件层级提供（V2） | 属性 |
| `@Consumer` | 跨组件层级消费（V2） | 属性 |
| `@Monitor` | 状态变量修改监听 | 方法 |
| `@Computed` | 计算属性（通常用于 getter） | 方法/属性 |
| `@Type` | 标记类型 | 类 |
| `@ObservedV2` | 类对象观测（V2） | 类 |
| `@Trace` | 属性追踪（V2） | 类属性 |

### 🔷 UI 构建装饰器
| 装饰器 | 说明 | 适用范围 |
|--------|------|----------|
| `@Builder` | 自定义构建函数 | 函数/方法 |
| `@BuilderParam` | 引用 @Builder 函数，类似插槽 | 属性 |
| `@LocalBuilder` | 维持组件关系的局部构建器 | 方法 |
| `@Styles` | 定义组件重用样式 | 函数/方法 |
| `@Extend` | 扩展原生组件样式 | 函数 |
| `@AnimatableExtend` | 可动画扩展样式 | 函数 |

### 🔷 其他装饰器
| 装饰器 | 说明 | 适用范围 |
|--------|------|----------|
| `@Require` | 校验构造传参，确保必传参数 | 属性 |
| `@Reusable` | 标记组件可复用，优化性能 | 组件 |
| `@Concurrent` | 标记并发函数（用于 TaskPool） | 函数 |

## 📚 使用示例

### 状态管理 V1

```typescript
@Observed
class UserInfo {
  name: string = '';
  @Track age: number = 0;  // 精细化观测
}

@Entry
@Component
struct MainPage {
  @State message: string = 'Hello';
  @Provide('theme') theme: string = 'light';
  @StorageLink('count') count: number = 0;
  
  @State @Watch('onDataChange') data: string = '';
  
  onDataChange() {
    console.log('Data changed');
  }
  
  build() {
    Column() {
      ChildComponent({ title: this.message })
    }
  }
}

@Component
struct ChildComponent {
  @Prop title: string = '';
  @Link shared: number = 0;
  @Consume('theme') theme: string = '';
  @Require @Prop required: string = '';  // 必传参数
  
  build() {
    Text(this.title)
  }
}
```

### 状态管理 V2

```typescript
@ObservedV2
class UserProfile {
  @Trace name: string = '';
  @Trace age: number = 0;
}

@ComponentV2
struct ModernComponent {
  @Local localState: string = '';
  @Param inputValue: string = '';
  @Provider('config') config: string = '{}';
  @Event onChange: (value: string) => void = () => {};
  
  @Monitor('localState')
  onStateChange() {
    console.log('State changed');
  }
  
  build() {
    Text(this.localState)
  }
}
```

### UI 构建装饰器

```typescript
// 全局 Builder
@Builder
function CustomButton(text: string) {
  Button(text)
    .width('100%')
    .height(50)
}

// 组件内 Builder
@Component
struct Container {
  @BuilderParam content: () => void = () => {};
  
  @Builder
  localBuilder() {
    Text('Local content')
  }
  
  build() {
    Column() {
      this.content();
      this.localBuilder();
    }
  }
}

// Styles 装饰器
@Styles
function globalStyles() {
  .width('100%')
  .padding(20)
}

// Extend 装饰器
@Extend(Text)
function fancyText(color: Color) {
  .fontColor(color)
  .fontSize(18)
  .fontWeight(FontWeight.Bold)
}
```

### 组件复用与并发

```typescript
// 可复用组件
@Reusable
@Component
struct ReusableItem {
  @State data: string = '';
  
  build() {
    Text(this.data)
  }
}

// 并发函数
@Concurrent
function processData(data: string[]): string[] {
  return data.map(item => item.toUpperCase());
}
```

## 🎯 版本兼容性

- **V1 装饰器**: 所有 HarmonyOS 版本
- **V2 装饰器**: API version 12 及以上（`@ComponentV2`, `@Local`, `@Param`, `@Monitor`, `@Computed` 等）
- **@Reusable**: API version 10 及以上
- **@Concurrent**: API version 9 及以上

## 📖 参考文档

- [HarmonyOS 状态管理官方文档](https://developer.huawei.com/consumer/cn/doc/harmonyos-guides/arkts-component-state-management)
- [@Builder 装饰器](https://developer.huawei.com/consumer/cn/doc/harmonyos-guides/arkts-builder)
- [@Styles 和 @Extend 装饰器](https://developer.huawei.com/consumer/cn/doc/harmonyos-guides/arkts-style)
- [@ComponentV2 装饰器](https://developer.huawei.com/consumer/cn/doc/harmonyos-guides/arkts-new-componentv2)

## ✅ 测试文件

- `examples/decorators_complete.ets` - 完整装饰器使用示例
- `test/test_new_decorators.ets` - 新增装饰器测试
- `examples/advanced_state_management.ets` - 高级状态管理示例

## 🚀 特性亮点

1. ✅ **完整支持所有官方装饰器**（40+ 种）
2. ✅ **支持装饰器组合使用** (`@State @Watch`, `@Require @Prop`)
3. ✅ **支持装饰器参数** (`@Provide('key')`, `@Extend(Component)`)
4. ✅ **兼容 V1 和 V2 两套状态管理体系**
5. ✅ **支持全局和组件内装饰器函数**
6. ✅ **正确解析装饰器作用域和语义**

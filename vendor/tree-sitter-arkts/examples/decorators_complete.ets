// 完整的ArkTS装饰器测试文件
// 测试所有支持的装饰器类型

// ============================================
// 状态管理 V1 装饰器
// ============================================

// @Observed - 类对象观测
@Observed
class UserInfo {
  name: string = '';
  age: number = 0;
  
  @Track email: string = '';  // @Track - 精细化属性观测
}

// @Entry + @Component - 基础组件
@Entry
@Component
struct MainPage {
  @State message: string = 'Hello';
  @State count: number = 0;
  @State userInfo: UserInfo = new UserInfo();
  
  @Provide('theme') theme: string = 'light';
  @Provide('locale') locale: string = 'zh-CN';
  
  @StorageLink('globalCount') globalCount: number = 0;
  @StorageProp('appName') appName: string = 'MyApp';
  
  @State @Watch('onCountChange') watchCount: number = 0;
  
  onCountChange() {
    console.log('Count changed');
  }
  
  build() {
    Column() {
      Text(this.message)
      ChildComponent({ 
        title: this.message,
        shared: this.count 
      })
    }
  }
}

// @Component - 子组件
@Component
struct ChildComponent {
  @Prop title: string = '';
  @Link shared: number = 0;
  @Consume('theme') theme: string = '';
  @ObjectLink userInfo: UserInfo = new UserInfo();
  
  @LocalStorageLink('localData') localData: string = '';
  @LocalStorageProp('settings') settings: string = '{}';
  
  @Require @Prop required: string = '';  // @Require - 校验构造传参
  
  @BuilderParam content: () => void = () => {};  // @BuilderParam - 引用@Builder函数
  
  build() {
    Column() {
      Text(this.title)
    }
  }
}

// @Reusable - 组件复用
@Reusable
@Component
struct ReusableItem {
  @State data: string = '';
  
  build() {
    Text(this.data)
  }
}

// ============================================
// 状态管理 V2 装饰器
// ============================================

// @ObservedV2 - V2版本的对象观测
@ObservedV2
class UserProfile {
  name: string = '';
  @Trace age: number = 0;  // @Trace - V2版本的属性追踪
}

// @ComponentV2 - V2自定义组件
@ComponentV2
struct ModernComponent {
  @Local localState: string = '';  // @Local - 组件内部状态(V2)
  @Param inputValue: string = '';  // @Param - 组件外部输入(V2)
  @Once initialValue: string = ''; // @Once - 初始化同步一次
  
  @Provider('config') config: string = '{}';  // @Provider - 跨组件层级提供(V2)
  @Consumer('settings') settings: string = '{}';  // @Consumer - 跨组件层级消费(V2)
  
  @Event onChange: (value: string) => void = () => {};  // @Event - 规范组件输出
  
  // @Computed装饰器通常用于getter方法，这里暂时注释
  // @Computed
  // get displayValue(): string {
  //   return this.localState + this.inputValue;
  // }
  
  @Monitor('localState')
  onLocalStateChange() {  // @Monitor - 状态变量修改监听
    console.log('Local state changed');
  }
  
  build() {
    Column() {
      Text(this.localState)
    }
  }
}

// @Type - 标记类型
@Type
class TypedData {
  value: string = '';
}

// ============================================
// UI构建装饰器
// ============================================

// @Builder - 自定义构建函数（全局）
@Builder
function GlobalBuilder(title: string) {
  Text(title)
    .fontSize(20)
    .fontWeight(FontWeight.Bold)
}

// @Builder export - 导出的构建函数
@Builder
export function ExportedBuilder(content: string) {
  Row() {
    Text(content)
  }
}

// @Styles - 定义组件重用样式（全局）
@Styles
function globalStyles() {
  .width('100%')
  .height(50)
  .backgroundColor(Color.White)
}

// @Extend - 扩展原生组件样式
@Extend(Text)
function fancyText(color: Color) {
  .fontColor(color)
  .fontSize(18)
  .fontWeight(FontWeight.Bold)
}

// @AnimatableExtend - 可动画扩展
@AnimatableExtend(Text)
function animatableText(size: number) {
  .fontSize(size)
}

// @LocalBuilder - 维持组件关系
@Component
struct BuilderContainer {
  @LocalBuilder
  localContent() {
    Text('Local Builder Content')
  }
  
  @Styles
  localStyles() {
    .padding(10)
    .margin(5)
  }
  
  build() {
    Column() {
      this.localContent();
    }
  }
}

// ============================================
// 其他装饰器
// ============================================

// @Concurrent - 并发函数标记（用于TaskPool）
@Concurrent
function concurrentTask(data: string): string {
  return data.toUpperCase();
}

// 综合使用示例
@Entry
@Component
struct CompleteExample {
  @State items: string[] = ['A', 'B', 'C'];
  @Provide('appTheme') appTheme: string = 'dark';
  
  @Builder
  itemBuilder(item: string) {
    Text(item)
      .fancyText(Color.Blue)
  }
  
  @Styles
  containerStyles() {
    .width('100%')
    .padding(20)
  }
  
  build() {
    Column() {
      ForEach(this.items, (item: string) => {
        this.itemBuilder(item)
      })
    }
    .containerStyles()
  }
}

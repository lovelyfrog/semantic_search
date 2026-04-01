# 动态 import() 表达式支持

## 概述

tree-sitter-arkts 现已支持 ES2020+ 的动态 `import()` 表达式语法，该特性允许在运行时动态加载模块。

## 语法支持

### 基本语法

```typescript
import(moduleSpecifier)
```

- **moduleSpecifier**: 可以是字符串字面量或返回字符串的表达式

## 支持的用法

### 1. 顶层独立调用

```typescript
import('@ohos/login/src/main/ets/pages/LoginPage');
```

### 2. 赋值给变量

```typescript
const loginModule = import('@ohos/login/src/main/ets/pages/LoginPage');
```

### 3. async/await 模式

```typescript
async function loadLoginModule() {
  const LoginModule = await import('@ohos/login/src/main/ets/pages/LoginPage');
  return LoginModule.default;
}
```

### 4. Promise 链式调用

```typescript
import('@ohos/user/UserManager')
  .then(module => {
    console.log('Module loaded:', module);
  })
  .catch(err => {
    console.error('Failed to load:', err);
  });
```

### 5. 动态路径（使用变量）

```typescript
const modulePath = '@ohos/common/utils';
const utilsModule = import(modulePath);
```

### 6. 条件导入

```typescript
const isDev = true;
const config = isDev 
  ? import('./config.dev') 
  : import('./config.prod');
```

### 7. 批量导入

```typescript
const modules = [
  import('./module1'),
  import('./module2'),
  import('./module3')
];

// 使用 Promise.all
Promise.all([
  import('./ComponentA'),
  import('./ComponentB'),
  import('./ComponentC')
]).then(([A, B, C]) => {
  console.log('All loaded');
});
```

### 8. 在类方法中使用

```typescript
class DynamicLoader {
  async loadComponent(path: string) {
    const component = await import(path);
    return component;
  }
  
  loadMultiple(paths: string[]) {
    return Promise.all(paths.map(p => import(p)));
  }
}
```

## AST 结构

动态 `import()` 在语法树中被解析为 `import_expression` 节点：

```
(import_expression
  (expression
    (string_literal)))
```

## 与静态 import 的区别

| 特性 | 静态 import | 动态 import() |
|------|------------|--------------|
| 语法位置 | 仅顶层 | 任意位置 |
| 加载时机 | 编译时 | 运行时 |
| 条件导入 | ❌ | ✅ |
| 变量路径 | ❌ | ✅ |
| 返回值 | - | Promise |
| 用途 | 静态依赖 | 按需加载、代码分割 |

## 测试文件

- `test/test_import_expression.ets` - 完整的使用示例
- `test/test_user_import.ets` - 常见场景测试
- `test/test_dynamic_import.ets` - 边界情况测试

## 版本历史

- **v0.1.7** (2025-10-20): 新增动态 `import()` 表达式支持

## 相关文档

- [MDN - Dynamic Import](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Operators/import)
- [TC39 Proposal](https://github.com/tc39/proposal-dynamic-import)

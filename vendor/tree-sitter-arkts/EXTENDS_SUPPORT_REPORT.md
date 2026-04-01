# Extends 语法支持情况报告

## ✅ 已完全支持的场景

### 1. 类继承 (Class Extends)

#### 1.1 基本类继承
```typescript
class Dog extends Animal {
  breed: string;
}
```
✅ **状态**: 完全支持  
**规则**: `class_declaration` 中的 `optional(seq('extends', $.type_annotation))`

#### 1.2 泛型类继承
```typescript
class NumberContainer extends Container<number> {
  double(): number { return this.value * 2; }
}
```
✅ **状态**: 完全支持  
**规则**: extends 后面支持 `$.type_annotation`，包括泛型类型

#### 1.3 类继承 + implements
```typescript
class Component extends BaseComponent implements Lifecycle {
  mounted(): void {}
}
```
✅ **状态**: 完全支持  
**规则**: `class_declaration` 同时支持 `extends` 和 `implements_clause`

#### 1.4 装饰器 + 类继承
```typescript
@Observed
class UserModel extends BaseModel {
  name: string;
}
```
✅ **状态**: 完全支持  
**规则**: `class_declaration` 支持装饰器 + extends

#### 1.5 导出 + 类继承
```typescript
export class UserService extends BaseService {
  getUsers(): void {}
}
```
✅ **状态**: 完全支持  
**规则**: `export_declaration` 和 `decorated_export_declaration` 中都支持类继承

#### 1.6 抽象类继承
```typescript
abstract class Shape {
  abstract getArea(): number;
}

class Circle extends Shape {
  getArea(): number { return 3.14; }
}
```
✅ **状态**: 完全支持  
**规则**: `class_declaration` 支持 `optional('abstract')`


### 2. 接口继承 (Interface Extends)

#### 2.1 单一接口继承
```typescript
interface User extends Entity {
  name: string;
}
```
✅ **状态**: 完全支持  
**规则**: `interface_declaration` 中的 `optional($.extends_clause)`

#### 2.2 多重接口继承
```typescript
interface Post extends Entity, Timestamped, Deletable {
  title: string;
}
```
✅ **状态**: 完全支持  
**规则**: `extends_clause` 使用 `commaSep()` 支持多个接口

#### 2.3 泛型接口继承
```typescript
interface UserRepository extends Repository<User> {
  findByEmail(email: string): User;
}
```
✅ **状态**: 完全支持  
**规则**: `extends_clause` 支持 `$.generic_type`

#### 2.4 带泛型参数的接口继承
```typescript
interface Pageable<T> extends BaseParams {
  items: T[];
}
```
✅ **状态**: 完全支持  
**规则**: `interface_declaration` 同时支持 `type_parameters` 和 `extends_clause`

#### 2.5 导出 + 接口继承
```typescript
export interface GetUserParams extends BaseParams {
  includeDetails: boolean;
}
```
✅ **状态**: 完全支持  
**规则**: `export_declaration` 中支持接口继承


### 3. 泛型约束 (Type Parameter Extends)

#### 3.1 基本泛型约束
```typescript
function getName<T extends { name: string }>(obj: T): string {
  return obj.name;
}
```
✅ **状态**: 完全支持  
**规则**: `type_parameter` 中的 `optional(seq('extends', $.type_annotation))`

#### 3.2 多个泛型参数约束
```typescript
function merge<T extends object, U extends object>(obj1: T, obj2: U) {
  return { ...obj1, ...obj2 };
}
```
✅ **状态**: 完全支持  
**规则**: 每个 `type_parameter` 都可以有独立的 `extends` 约束

#### 3.3 类中的泛型约束
```typescript
class DataStore<T extends { id: string }> {
  private items: T[];
}
```
✅ **状态**: 完全支持  
**规则**: `class_declaration` 的 `type_parameters` 支持约束

#### 3.4 接口中的泛型约束
```typescript
interface Comparable<T extends number | string> {
  compareTo(other: T): number;
}
```
✅ **状态**: 完全支持  
**规则**: `interface_declaration` 的 `type_parameters` 支持约束


### 4. 条件类型 (Conditional Types)

#### 4.1 基本条件类型
```typescript
type IsString<T> = T extends string ? true : false;
```
✅ **状态**: 完全支持  
**规则**: `conditional_type` - `seq($.primary_type, 'extends', $.type_annotation, '?', $.type_annotation, ':', $.type_annotation)`

#### 4.2 条件类型过滤
```typescript
type NonNullable<T> = T extends null | undefined ? never : T;
```
✅ **状态**: 完全支持  
**规则**: 条件类型支持联合类型和特殊类型（never）


---

## ❌ 不支持的高级场景

### 1. keyof 类型操作符
```typescript
function getProperty<T, K extends keyof T>(obj: T, key: K): T[K] {
  return obj[key];
}
```
❌ **状态**: 不支持  
**原因**: 缺少 `keyof` 关键字支持  
**影响**: 无法使用映射类型的键约束

### 2. infer 类型推断
```typescript
type ExtractArrayType<T> = T extends Array<infer U> ? U : never;
```
❌ **状态**: 不支持  
**原因**: 缺少 `infer` 关键字支持  
**影响**: 无法在条件类型中推断类型

### 3. 复杂的索引访问类型
```typescript
type GetReturnType<T> = T extends (...args: any) => infer R ? R : never;
```
❌ **状态**: 不支持  
**原因**: 需要 `infer` 支持  
**影响**: 无法提取函数返回类型等高级类型操作


---

## 📊 支持情况统计

| 场景分类 | 支持项 | 不支持项 | 支持率 |
|---------|--------|----------|--------|
| 类继承 | 6/6 | 0/6 | 100% |
| 接口继承 | 5/5 | 0/5 | 100% |
| 泛型约束 | 4/4 | 0/4 | 100% |
| 条件类型 | 2/2 | 0/2 | 100% |
| 高级类型操作 | 0/3 | 3/3 | 0% |
| **总计** | **17/20** | **3/20** | **85%** |


---

## 🎯 extends 语法规则分布

### 1. class_declaration
```javascript
class_declaration: $ => seq(
  repeat($.decorator),
  optional('abstract'),
  'class',
  $.identifier,
  optional($.type_parameters),
  optional(seq('extends', $.type_annotation)),  // ✅ 类继承
  optional($.implements_clause),
  $.class_body
),
```

### 2. interface_declaration
```javascript
interface_declaration: $ => seq(
  'interface',
  $.identifier,
  optional($.type_parameters),
  optional($.extends_clause),  // ✅ 接口继承
  $.object_type
),
```

### 3. extends_clause
```javascript
extends_clause: $ => seq(
  'extends',
  commaSep(choice(
    $.identifier,
    $.generic_type  // ✅ 支持泛型接口继承
  ))
),
```

### 4. type_parameter
```javascript
type_parameter: $ => seq(
  $.identifier,
  optional(seq('extends', $.type_annotation)),  // ✅ 泛型约束
  optional(seq('=', $.type_annotation))
),
```

### 5. conditional_type
```javascript
conditional_type: $ => prec.right(1, seq(
  $.primary_type,
  'extends',  // ✅ 条件类型
  $.type_annotation,
  '?',
  $.type_annotation,
  ':',
  $.type_annotation
)),
```

### 6. decorated_export_declaration
```javascript
// export class 中的 extends
seq(
  optional('abstract'),
  'class',
  $.identifier,
  optional($.type_parameters),
  optional(seq('extends', $.type_annotation)),  // ✅ 导出类继承
  optional($.implements_clause),
  $.class_body
),
```


---

## ✅ 结论

### 核心 extends 功能 - 100% 支持

tree-sitter-arkts 对 `extends` 关键字的**核心用法**已经实现了**完整支持**：

1. ✅ **类继承** - 包括普通类、抽象类、泛型类、装饰器类
2. ✅ **接口继承** - 包括单一继承、多重继承、泛型接口继承
3. ✅ **泛型约束** - 支持在类、接口、函数中使用泛型约束
4. ✅ **条件类型** - 支持基本的条件类型判断

### 高级 TypeScript 特性 - 部分不支持

以下高级特性不支持（但不影响 ArkTS 核心开发）：

- ❌ `keyof` 操作符
- ❌ `infer` 类型推断
- ❌ 高级映射类型

这些特性主要用于高级类型编程，在 ArkTS 日常开发中使用较少。

### 建议

对于 ArkTS 语法解析器来说，当前的 `extends` 支持已经**非常完善**，能够覆盖：
- ✅ 所有面向对象编程场景（类继承、接口继承）
- ✅ 所有泛型编程场景（泛型约束）
- ✅ 基础类型系统（条件类型）
- ✅ 所有导出和装饰器组合场景

**推荐优先级**：如需扩展，建议按以下顺序：
1. 低优先级：`keyof` - 用于高级类型操作
2. 低优先级：`infer` - 用于类型推断
3. 极低优先级：其他高级映射类型操作符

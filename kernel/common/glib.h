//
// 内核全局通用库
// Created by longjin on 2022/1/22.
//

#pragma once

//引入对bool类型的支持
#include <stdbool.h>

#define NULL 0

#define sti() __asm__ __volatile__("sti\n\t" :: \
                                       : "memory") //开启外部中断
#define cli() __asm__ __volatile__("cli\n\t" :: \
                                       : "memory") //关闭外部中断
#define nop() __asm__ __volatile__("nop\n\t")

//内存屏障
#define io_mfence() __asm__ __volatile__("mfence\n\t" :: \
                                             : "memory") // 在mfence指令前的读写操作当必须在mfence指令后的读写操作前完成。
#define io_sfence() __asm__ __volatile__("sfence\n\t" :: \
                                             : "memory") // 在sfence指令前的写操作当必须在sfence指令后的写操作前完成
#define io_lfence() __asm__ __volatile__("lfence\n\t" :: \
                                             : "memory") // 在lfence指令前的读操作当必须在lfence指令后的读操作前完成。

// 定义类型的缩写
typedef unsigned long ul;
typedef unsigned long long ull;
typedef long long ll;

#define ABS(x) ((x) > 0 ? (x) : -(x))   // 绝对值

// 四舍五入成整数
ul round(double x)
{
    return (ul)(x+0.5);
}


//链表数据结构
struct List
{
    struct List *prev, *next;
};

//初始化循环链表
static inline void list_init(struct List *list)
{
    list->next = list;
    list->prev = list;
}

static inline void list_add(struct List *entry, struct List *node)
{
    /**
     * @brief 将node插入到entry后面
     * @param entry 给定的节点
     * @param node 待插入的节点
     */

    node->next = entry->next;
    node->next->prev = node;
    node->prev = entry;
    entry->next = node;
}

static inline void list_append(struct List *entry, struct List *node)
{
    /**
     * @brief 将node添加到给定的list的结尾(也就是当前节点的前面)
     * @param entry 列表的入口
     * @param node 待添加的节点
     */

    struct List *tail = entry->prev;
    list_add(tail, node);
}

static inline void list_del(struct List *entry)
{
    /**
     * @brief 从列表中删除节点
     * @param entry 待删除的节点
     */

    entry->prev->next = entry->next;
    entry->next = entry->prev;
}

static inline bool list_empty(struct List *entry)
{
    /**
     * @brief 判断循环链表是否为空
     * @param entry 入口
     */

    if (entry->prev == entry->next)
        return true;
    else
        return false;
}

//计算字符串的长度（经过测试，该版本比采用repne/scasb汇编的运行速度快16.8%左右）
static inline int strlen(char *s)
{
    register int __res = 0;
    while (s[__res] != '\0')
    {
        ++__res;
    }
    return __res;
}



inline void *memset(void *dst, unsigned char C, ul Count)
{
    int d0, d1;
    unsigned long tmp = C * 0x0101010101010101UL;
    __asm__ __volatile__("cld	\n\t"
                         "rep	\n\t"
                         "stosq	\n\t"
                         "testb	$4, %b3	\n\t"
                         "je	1f	\n\t"
                         "stosl	\n\t"
                         "1:\ttestb	$2, %b3	\n\t"
                         "je	2f\n\t"
                         "stosw	\n\t"
                         "2:\ttestb	$1, %b3	\n\t"
                         "je	3f	\n\t"
                         "stosb	\n\t"
                         "3:	\n\t"
                         : "=&c"(d0), "=&D"(d1)
                         : "a"(tmp), "q"(Count), "0"(Count / 8), "1"(dst)
                         : "memory");
    return dst;
}